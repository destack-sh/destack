import { and, eq } from "@destack/db";
import { RecordProvenance } from "@destack/model/source";
import { createRequestId, fingerprintRequest } from "@destack/service/request";
import { ServiceError } from "@destack/service/error";
import { v7 } from "uuid";
import { SettingAssignment } from "../setting/assignment.ts";
import { SettingPolicy } from "../setting/policy.ts";
import type { Setting } from "../setting/setting.ts";
import type { SettingAssignmentDefinition, SettingPolicyDefinition } from "../declare/index.ts";
import { settingAssignment, settingPolicy, settingSource } from "../stack/index.ts";
import { SettingStore, type SettingWrite } from "./store.ts";
import { assignmentKey, decodeAssignment, getAssignment, mutateAssignment } from "./assignment.ts";
import { mutatePolicy, decodePolicy } from "./policy.ts";
import { sourceKey } from "./source.ts";

/** A complete authorized source revision and its desired settings. */
export interface SettingReconciliation {
    /** Retry identity for the whole atomic application. */
    readonly requestId: string;
    /** Previously applied reconciliation number, or null for first application. */
    readonly expectedRevision: number | null;
    /** Declaring source and immutable revision, shared by each named definition. */
    readonly source: {
        [Kind in RecordProvenance["kind"]]: Omit<Extract<RecordProvenance, { kind: Kind }>, "name">;
    }[RecordProvenance["kind"]];
    /** Complete assignment collection; omission leaves that collection unchanged. */
    readonly assignments?: Readonly<Record<string, SettingAssignmentDefinition>>;
    /** Complete policy collection; omission leaves that collection unchanged. */
    readonly policies?: Readonly<Record<string, SettingPolicyDefinition>>;
}

/** Apply a complete source revision after the host authorizes every affected target and authority. */
export async function reconcileSettings(
    store: SettingStore,
    input: SettingReconciliation,
    declarations: readonly Setting[],
    context: SettingWrite,
): Promise<{ revision: number; assignments: SettingAssignment[]; policies: SettingPolicy[] }> {
    // claim the whole source revision before changing any individual record
    const source = RecordProvenance.parse({ ...input.source, name: "settings" });
    const scope = sourceKey(source);
    const request = {
        caller: JSON.stringify(context.subject),
        scope,
        procedure: "setting.reconcile",
        requestId: input.requestId,
    };
    const digest = Buffer.from(await fingerprintRequest(input)).toString("hex");
    const result = await store.database.transaction(async (database) => {
        // replay a completed request with the same fingerprint
        const claim = await store.requests.begin(
            database,
            request,
            { digest },
            (stored) => stored === digest,
        );
        if (claim.kind === "replay") {
            const value = claim.value as {
                revision: number;
                assignments: unknown[];
                policies: unknown[];
            };

            return {
                revision: value.revision,
                assignments: value.assignments.map((record) => SettingAssignment.parse(record)),
                policies: value.policies.map((record) => SettingPolicy.parse(record)),
            };
        }

        // serialize complete applications and reject stale source revisions before editing records
        const revision = input.expectedRevision === null ? 1 : input.expectedRevision + 1;
        const written =
            input.expectedRevision === null
                ? await database
                      .insert(settingSource)
                      .values({ key: scope, revision })
                      .onConflictDoNothing()
                      .returning({ key: settingSource.key })
                : await database
                      .update(settingSource)
                      .set({ revision })
                      .where(
                          and(
                              eq(settingSource.key, scope),
                              eq(settingSource.revision, input.expectedRevision),
                          ),
                      )
                      .returning({ key: settingSource.key });
        if (written.length !== 1) {
            throw new ServiceError("CONFLICT", { message: "setting source revision has changed" });
        }

        // load current records attributed to this source, including detachments
        const transaction = new SettingStore(database);
        const previousAssignments = (
            await database
                .select()
                .from(settingAssignment)
                .where(eq(settingAssignment.source, scope))
        ).map(decodeAssignment);
        const previousPolicies = (
            await database.select().from(settingPolicy).where(eq(settingPolicy.source, scope))
        ).map(decodePolicy);
        const assignments: SettingAssignment[] = [];
        const policies: SettingPolicy[] = [];

        // delete assignments removed or moved by the source
        for (const previous of previousAssignments) {
            const provenance = previous.provenance!;
            const desired = input.assignments?.[provenance.name];
            const moved =
                desired &&
                assignmentKey(desired.setting, desired.target) !==
                    assignmentKey(previous.setting, previous.target);
            if (
                input.assignments !== undefined &&
                (!desired || moved) &&
                previous.detachedAt === null
            ) {
                await mutateAssignment(
                    transaction,
                    undefined,
                    {
                        requestId: createRequestId(),
                        setting: previous.setting,
                        target: previous.target,
                        expectedRevision: previous.revision,
                    },
                    {
                        ...context,
                        provenance: RecordProvenance.parse({
                            ...input.source,
                            name: provenance.name,
                        }),
                    },
                    "reset",
                );
            }
        }

        // apply current definitions, preserving records explicitly detached from their source
        for (const [name, definition] of Object.entries(input.assignments ?? {})) {
            if (
                previousAssignments.some(
                    (record) => record.provenance?.name === name && record.detachedAt !== null,
                )
            ) {
                continue;
            }
            const previous = await getAssignment(database, definition.setting, definition.target);
            assignments.push(
                await mutateAssignment(
                    transaction,
                    selectSetting(definition.setting, declarations),
                    {
                        requestId: createRequestId(),
                        setting: definition.setting,
                        target: definition.target,
                        expectedRevision: previous?.revision ?? null,
                        value: definition.value,
                    },
                    { ...context, provenance: RecordProvenance.parse({ ...input.source, name }) },
                ),
            );
        }

        // delete policies removed by the source
        for (const previous of previousPolicies) {
            const name = previous.provenance!.name;
            if (
                input.policies !== undefined &&
                !input.policies[name] &&
                previous.detachedAt === null
            ) {
                await mutatePolicy(
                    transaction,
                    {
                        authority: previous.authority,
                        id: previous.id,
                        requestId: createRequestId(),
                        expectedRevision: previous.revision,
                        operation: "remove",
                    },
                    {
                        ...context,
                        provenance: RecordProvenance.parse({ ...input.source, name }),
                    },
                );
            }
        }
        for (const [name, definition] of Object.entries(input.policies ?? {})) {
            const previous = previousPolicies.find((record) => record.provenance?.name === name);
            if (previous?.detachedAt !== null && previous?.detachedAt !== undefined) {
                continue;
            }
            const setting = selectSetting(definition.setting, declarations);
            if (!setting.definition.schema.safeParse(definition.value).success) {
                throw new ServiceError("BAD_REQUEST", {
                    message: "policy value does not match its setting declaration",
                });
            }
            if (
                previous &&
                JSON.stringify(previous.authority) !== JSON.stringify(definition.authority)
            ) {
                throw new ServiceError("CONFLICT", {
                    message: "source policy authority cannot change",
                });
            }
            policies.push(
                (await mutatePolicy(
                    transaction,
                    {
                        authority: definition.authority,
                        id: previous?.id ?? SettingPolicy.shape.id.parse(`setting-policy-${v7()}`),
                        requestId: createRequestId(),
                        expectedRevision: previous?.revision ?? null,
                        definition,
                        operation: "set",
                    },
                    { ...context, provenance: RecordProvenance.parse({ ...input.source, name }) },
                ))!,
            );
        }

        // complete the request with the applied records
        const result = { revision, assignments, policies };
        await store.requests.complete(database, request, result);

        return result;
    });
    store.notify();

    return result;
}

/** Require the exact declaration selected by the applying host. */
function selectSetting(
    reference: SettingAssignment["setting"],
    declarations: readonly Setting[],
): Setting {
    const setting = declarations.find(
        (candidate) =>
            candidate.reference.packageId === reference.packageId &&
            candidate.reference.name === reference.name,
    );
    if (!setting) {
        throw new ServiceError("NOT_FOUND", {
            message: "source setting declaration is unavailable",
        });
    }

    return setting;
}

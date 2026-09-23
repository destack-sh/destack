import { and, asc, eq, gt, type DatabaseConnection } from "@destack/db";
import { ServiceError } from "@destack/service/error";
import { Page } from "@destack/service/page";
import { fingerprintRequest } from "@destack/service/request";
import { schema } from "@destack/schema";
import { SettingPolicy } from "../setting/policy.ts";
import { SettingAuthority } from "../setting/target.ts";
import type { SettingPolicyDefinition } from "../declare/policy.ts";
import type { SettingStore, SettingWrite } from "./store.ts";
import { settingPolicy } from "../stack/index.ts";
import { changePolicy } from "./audit.ts";
import { sameSource, sourceKey } from "./source.ts";

/** Read one policy under its exact administrative authority. */
export async function getPolicy(
    database: DatabaseConnection,
    authority: SettingAuthority,
    id: string,
): Promise<SettingPolicy | null> {
    const row = await database
        .select()
        .from(settingPolicy)
        .where(and(eq(settingPolicy.id, id), policyAuthority(authority)))
        .get();

    return row ? decodePolicy(row) : null;
}

/** Read a bounded authority-scoped page. */
export async function listPolicy(
    database: DatabaseConnection,
    authority: SettingAuthority,
    input: { limit: number; cursor?: string },
) {
    const selected = JSON.stringify(SettingAuthority.parse(authority));
    const page = new Page(input, ["policy", selected], schema.string());
    const rows = await database
        .select()
        .from(settingPolicy)
        .where(
            and(
                policyAuthority(authority),
                page.after === undefined ? undefined : gt(settingPolicy.id, page.after),
            ),
        )
        .orderBy(asc(settingPolicy.id))
        .limit(page.limit + 1);
    const result = page.result(rows, (row) => row.id);

    return { ...result, items: result.items.map(decodePolicy) };
}

/** A conditional policy replacement, removal or source detachment. */
export interface PolicyEdit {
    /** Authority verified by the host before mutation. */
    readonly authority: SettingAuthority;
    /** Persistent policy identity. */
    readonly id: SettingPolicy["id"];
    /** Retry identity retained across delivery attempts. */
    readonly requestId: string;
    /** Revision observed by the editor, or null when creating. */
    readonly expectedRevision: string | null;
    /** Replacement contents; omitted for removal or detachment. */
    readonly definition?: SettingPolicyDefinition;
    /** Requested lifecycle transition. */
    readonly operation: "set" | "remove" | "detach";
}

/** Commit a policy transition with optimistic concurrency, replay protection and audit. */
export async function mutatePolicy(
    store: SettingStore,
    input: PolicyEdit,
    context: SettingWrite,
): Promise<SettingPolicy | null> {
    const request = {
        caller: JSON.stringify(context.subject),
        scope: JSON.stringify(input.authority),
        procedure: `policy.${input.operation}`,
        requestId: input.requestId,
    };
    const digest = Buffer.from(
        await fingerprintRequest({ input, provenance: context.provenance ?? null }),
    ).toString("hex");
    const result = await store.database.transaction(async (transaction) => {
        const claim = await store.requests.begin(
            transaction,
            request,
            { digest },
            (stored) => stored === digest,
        );
        if (claim.kind === "replay") {
            return claim.value === null ? null : SettingPolicy.parse(claim.value);
        }
        const previous = await getPolicy(transaction, input.authority, input.id);
        if ((previous?.revision ?? null) !== input.expectedRevision) {
            throw new ServiceError("CONFLICT", { message: "setting policy revision has changed" });
        }
        if (!previous && input.operation !== "set") {
            throw new ServiceError("NOT_FOUND");
        }
        if (
            previous?.provenance &&
            previous.detachedAt === null &&
            input.operation !== "detach" &&
            !sameSource(previous.provenance, context.provenance)
        ) {
            throw new ServiceError("CONFLICT", { message: "setting policy is managed by source" });
        }

        // preserve explicit detachment and reject adoption of independently managed policies
        if (context.provenance && previous) {
            if (!sameSource(previous.provenance, context.provenance)) {
                throw new ServiceError("CONFLICT", {
                    message: "setting policy belongs to another source",
                });
            }
            if (previous.detachedAt !== null) {
                throw new ServiceError("CONFLICT", {
                    message: "setting policy is detached from source",
                });
            }
        }
        if (input.operation === "detach" && !previous?.provenance) {
            throw new ServiceError("CONFLICT", { message: "setting policy has no source" });
        }

        // delete removed policies at their exact observed revision
        if (input.operation === "remove") {
            const deleted = await transaction
                .delete(settingPolicy)
                .where(
                    and(
                        eq(settingPolicy.id, input.id),
                        eq(settingPolicy.revision, previous!.revision),
                    ),
                )
                .returning({ id: settingPolicy.id });
            if (deleted.length !== 1) {
                throw new ServiceError("CONFLICT", {
                    message: "setting policy revision has changed",
                });
            }
            await context.audit.record(transaction, changePolicy, {
                targets: { policy: { type: "setting-policy", id: previous!.id } },
                details: {
                    setting: previous!.setting,
                    revision: previous!.revision,
                    operation: "remove",
                },
                outcome: "success",
            });
            await store.requests.complete(transaction, request, null);

            return null;
        }

        // retain identity and issue a fresh revision for each accepted edit
        const now = Date.now();
        const record = SettingPolicy.parse({
            ...previous,
            ...input.definition,
            authority: input.authority,
            id: input.id,
            revision: crypto.randomUUID(),
            provenance: context.provenance ?? previous?.provenance ?? null,
            detachedAt: input.operation === "detach" ? now : (previous?.detachedAt ?? null),
            createdAt: previous?.createdAt ?? now,
            updatedAt: now,
        });
        const written = previous
            ? await transaction
                  .update(settingPolicy)
                  .set(writePolicy(record))
                  .where(
                      and(
                          eq(settingPolicy.id, input.id),
                          eq(settingPolicy.revision, previous.revision),
                      ),
                  )
                  .returning({ id: settingPolicy.id })
            : await transaction
                  .insert(settingPolicy)
                  .values(writePolicy(record))
                  .onConflictDoNothing()
                  .returning({ id: settingPolicy.id });
        if (written.length !== 1) {
            throw new ServiceError("CONFLICT", { message: "setting policy revision has changed" });
        }
        await context.audit.record(transaction, changePolicy, {
            targets: { policy: { type: "setting-policy", id: record.id } },
            details: {
                setting: record.setting,
                revision: record.revision,
                operation: input.operation,
            },
            outcome: "success",
        });
        await store.requests.complete(transaction, request, record);

        return record;
    });
    store.notify(result ? [result.setting] : undefined);

    return result;
}

/** Select one exact administrative authority. */
export function policyAuthority(authority: SettingAuthority) {
    const column =
        authority.kind === "account"
            ? settingPolicy.accountId
            : authority.kind === "space"
              ? settingPolicy.spaceId
              : settingPolicy.hostId;
    const id =
        authority.kind === "account"
            ? authority.accountId
            : authority.kind === "space"
              ? authority.spaceId
              : authority.hostId;

    return and(eq(settingPolicy.authority, authority.kind), eq(column, id));
}

/** Decode a policy's administrative scope and optional refinement. */
export function decodePolicy(row: typeof settingPolicy.$inferSelect): SettingPolicy {
    const {
        authority,
        accountId,
        spaceId,
        hostId,
        installationId,
        source: _source,
        ...record
    } = row;
    const selection =
        authority === "account"
            ? { kind: authority, accountId }
            : authority === "space"
              ? { kind: authority, spaceId }
              : { kind: authority, hostId };

    return SettingPolicy.parse({
        ...record,
        authority: selection,
        ...(installationId === null ? {} : { installationId }),
    });
}

/** Map a policy to its indexed columns. */
export function writePolicy(record: SettingPolicy): typeof settingPolicy.$inferInsert {
    return {
        ...record,
        source: record.provenance ? sourceKey(record.provenance) : null,
        authority: record.authority.kind,
        accountId: record.authority.kind === "account" ? record.authority.accountId : null,
        spaceId: record.authority.kind === "space" ? record.authority.spaceId : null,
        hostId: record.authority.kind === "host" ? record.authority.hostId : null,
        installationId: record.installationId ?? null,
    };
}

import { and, asc, eq, gt, or, isNull, type DatabaseConnection } from "@destack/db";
import { Page } from "@destack/service/page";
import { schema } from "@destack/schema";
import type { PackageId } from "@destack/package";
import { ServiceError } from "@destack/service/error";
import { v7 } from "uuid";
import { fingerprintRequest } from "@destack/service/request";
import { SettingAssignment } from "../setting/assignment.ts";
import { SettingReference, type Setting } from "../setting/setting.ts";
import { SettingTarget } from "../setting/target.ts";
import type { SettingEdit } from "../setting/edit.ts";
import { settingAssignment } from "../stack/index.ts";
import { changeAssignment } from "./audit.ts";
import { sameSource, sourceKey } from "./source.ts";

import type { SettingStore, SettingWrite } from "./store.ts";

/** Read one exact target. */
export async function getAssignment(
    database: DatabaseConnection,
    setting: SettingReference,
    target: SettingTarget,
): Promise<SettingAssignment | null> {
    const row = await database
        .select()
        .from(settingAssignment)
        .where(
            and(
                eq(settingAssignment.packageId, setting.packageId),
                eq(settingAssignment.name, setting.name),
                assignmentTarget(target),
            ),
        )
        .get();

    return row ? decodeAssignment(row) : null;
}

/** Read a bounded exact-target page in stable identifier order. */
export async function listAssignment(
    database: DatabaseConnection,
    target: SettingTarget,
    limit: number,
    cursor?: string,
    packageId?: PackageId,
) {
    // bind the page to the target and package filter
    const selection = JSON.stringify(SettingTarget.parse(target));
    const page = new Page(
        { limit, cursor },
        ["assignment", selection, ...(packageId === undefined ? [] : [packageId])],
        schema.string(),
    );
    const rows = await database
        .select()
        .from(settingAssignment)
        .where(
            and(
                assignmentTarget(target),
                page.after === undefined ? undefined : gt(settingAssignment.id, page.after),
                packageId === undefined ? undefined : eq(settingAssignment.packageId, packageId),
            ),
        )
        .orderBy(asc(settingAssignment.id))
        .limit(limit + 1);
    const result = page.result(rows, (row) => row.id);

    return { ...result, items: result.items.map(decodeAssignment) };
}

/** Read a bounded refinement set within one database statement snapshot. */
export async function readAssignments(
    database: DatabaseConnection,
    settings: readonly SettingReference[],
    targets: readonly SettingTarget[],
): Promise<SettingAssignment[]> {
    if (settings.length === 0 || targets.length === 0) {
        return [];
    }
    const selections = targets.map(assignmentTarget);
    const declarations = settings.map((setting) =>
        and(
            eq(settingAssignment.packageId, setting.packageId),
            eq(settingAssignment.name, setting.name),
        ),
    );
    const rows = await database
        .select()
        .from(settingAssignment)
        .where(and(or(...declarations), or(...selections)));

    return rows.map(decodeAssignment);
}

/** Commit a conditional edit and replay identical requests without applying them twice. */
export function mutateAssignment(
    store: SettingStore,
    setting: Setting | undefined,
    input: SettingEdit,
    context: SettingWrite,
    operation: "reset",
): Promise<null>;
/** Replace or detach an assignment at its observed revision. */
export function mutateAssignment(
    store: SettingStore,
    setting: Setting | undefined,
    input: SettingEdit & { value?: schema.Infer<ReturnType<typeof schema.json>> },
    context: SettingWrite,
    operation?: "set" | "detach",
): Promise<SettingAssignment>;
/** Apply the requested transition in one database transaction. */
export async function mutateAssignment(
    store: SettingStore,
    setting: Setting | undefined,
    input: SettingEdit & { value?: schema.Infer<ReturnType<typeof schema.json>> },
    context: SettingWrite,
    operation: "set" | "reset" | "detach" = "set",
): Promise<SettingAssignment | null> {
    // require a declared target and valid replacement value before opening the transaction
    setting?.assertTarget(input.target);
    if (
        setting &&
        (input.setting.packageId !== setting.reference.packageId ||
            input.setting.name !== setting.reference.name)
    ) {
        throw new ServiceError("BAD_REQUEST", {
            message: "assignment does not match the selected declaration",
        });
    }
    if (operation === "set") {
        if (!setting) {
            throw new ServiceError("NOT_FOUND", {
                message: "setting declaration is unavailable",
            });
        }
        if (!setting.definition.schema.safeParse(input.value).success) {
            throw new ServiceError("BAD_REQUEST", {
                message: "setting value does not match its declaration",
            });
        }
    }
    const request = {
        caller: JSON.stringify(context.subject),
        scope: context.subject.authority,
        procedure: `assignment.${operation}`,
        requestId: input.requestId,
    };
    const digest = Buffer.from(
        await fingerprintRequest({ operation, input, provenance: context.provenance ?? null }),
    ).toString("hex");

    // commit request identity, assignment and audit as one transaction
    const result = await store.database.transaction(async (transaction) => {
        // replay a completed request with the same fingerprint
        const claim = await store.requests.begin(
            transaction,
            request,
            { digest },
            (stored) => stored === digest,
        );
        if (claim.kind === "replay") {
            return claim.value === null ? null : SettingAssignment.parse(claim.value);
        }
        const previous = await transaction
            .select()
            .from(settingAssignment)
            .where(
                and(
                    eq(settingAssignment.packageId, input.setting.packageId),
                    eq(settingAssignment.name, input.setting.name),
                    assignmentTarget(input.target),
                ),
            )
            .get();
        if (!setting && !previous) {
            throw new ServiceError("NOT_FOUND");
        }
        if ((previous?.revision ?? null) !== input.expectedRevision) {
            throw new ServiceError("CONFLICT", {
                message: "setting assignment revision has changed",
            });
        }
        if (
            previous?.provenance &&
            previous.detachedAt === null &&
            operation !== "detach" &&
            !sameSource(previous.provenance, context.provenance)
        ) {
            throw new ServiceError("CONFLICT", {
                message: "setting assignment is managed by source",
            });
        }

        // preserve explicitly detached records and reject source adoption of independent choices
        if (context.provenance && previous) {
            if (!sameSource(previous.provenance, context.provenance)) {
                throw new ServiceError("CONFLICT", {
                    message: "setting assignment belongs to another source",
                });
            }
            if (previous.detachedAt !== null) {
                throw new ServiceError("CONFLICT", {
                    message: "setting assignment is detached from source",
                });
            }
        }
        if (operation === "detach" && !previous?.provenance) {
            throw new ServiceError("CONFLICT", { message: "setting assignment has no source" });
        }

        // delete the exact assignment and audit the reset in the same transaction
        if (operation === "reset") {
            if (!previous) {
                await store.requests.complete(transaction, request, null);

                return null;
            }
            const deleted = await transaction
                .delete(settingAssignment)
                .where(
                    and(
                        eq(settingAssignment.id, previous.id),
                        eq(settingAssignment.revision, previous.revision),
                    ),
                )
                .returning({ id: settingAssignment.id });
            if (deleted.length !== 1) {
                throw new ServiceError("CONFLICT", {
                    message: "setting assignment revision has changed",
                });
            }
            await context.audit.record(transaction, changeAssignment, {
                targets: { assignment: { type: "setting-assignment", id: previous.id } },
                details: {
                    setting: input.setting,
                    revision: previous.revision,
                    operation: "reset",
                },
                outcome: "success",
            });
            await store.requests.complete(transaction, request, null);

            return null;
        }

        // retain identity while issuing a fresh revision for each accepted edit
        const now = Date.now();
        const record = SettingAssignment.parse({
            id: previous?.id ?? `setting-assignment-${v7()}`,
            setting: input.setting,
            target: input.target,
            value: operation === "detach" ? previous!.value : input.value,
            revision: crypto.randomUUID(),
            provenance: context.provenance ?? previous?.provenance ?? null,
            detachedAt: operation === "detach" ? now : (previous?.detachedAt ?? null),
            createdAt: previous?.createdAt ?? now,
            updatedAt: now,
        });

        // compare the observed record in SQL so concurrent writers cannot overwrite it
        const written = previous
            ? await transaction
                  .update(settingAssignment)
                  .set(writeAssignment(record))
                  .where(
                      and(
                          eq(settingAssignment.id, previous.id),
                          eq(settingAssignment.revision, previous.revision),
                      ),
                  )
                  .returning({ id: settingAssignment.id })
            : await transaction
                  .insert(settingAssignment)
                  .values(writeAssignment(record))
                  .onConflictDoNothing()
                  .returning({ id: settingAssignment.id });
        if (written.length !== 1) {
            throw new ServiceError("CONFLICT", {
                message: "setting assignment revision has changed",
            });
        }
        await context.audit.record(transaction, changeAssignment, {
            targets: { assignment: { type: "setting-assignment", id: record.id } },
            details: {
                setting: record.setting,
                revision: record.revision,
                operation,
            },
            outcome: "success",
        });
        await store.requests.complete(transaction, request, record);

        return record;
    });
    store.notify([input.setting]);

    return result;
}

/** Canonical identity independent of caller object member ordering. */
export function assignmentKey(setting: SettingReference, target: SettingTarget): string {
    return JSON.stringify([SettingReference.parse(setting), SettingTarget.parse(target)]);
}

/** Select one exact target using explicit indexed columns. */
export function assignmentTarget(target: SettingTarget) {
    const columns = targetColumns(target);
    return and(
        ...Object.entries(columns).map(([name, value]) => {
            const column = settingAssignment[name as keyof typeof columns];
            return value === null ? isNull(column) : eq(column, value);
        }),
    );
}

/** Encode an exact assignment selection into its database columns. */
function targetColumns(target: SettingTarget) {
    const location = "location" in target ? target.location : undefined;

    return {
        scope: target.kind,
        userAuthority: target.kind === "user" ? target.user.authority : null,
        userId: target.kind === "user" ? target.user.id : null,
        spaceId: location?.spaceId ?? null,
        hostId: target.kind === "host" ? target.hostId : null,
        consumerPackageId: target.kind === "user" ? (target.packageId ?? null) : null,
        installationId: location?.installationId ?? null,
        deviceId: target.kind === "user" ? (target.deviceId ?? null) : null,
    };
}

/** Decode an assignment's explicit scope and refinements. */
export function decodeAssignment(row: typeof settingAssignment.$inferSelect): SettingAssignment {
    // split the scope columns from the stored record
    const {
        source: _source,
        packageId,
        name,
        scope,
        userAuthority,
        userId,
        spaceId,
        hostId,
        consumerPackageId,
        installationId,
        deviceId,
        ...record
    } = row;
    const location =
        spaceId === null
            ? undefined
            : {
                  spaceId,
                  ...(installationId === null ? {} : { installationId }),
              };
    const target =
        scope === "user"
            ? {
                  kind: scope,
                  user: { kind: "user", authority: userAuthority, id: userId },
                  ...(consumerPackageId === null ? {} : { packageId: consumerPackageId }),
                  ...(location ? { location } : {}),
                  ...(deviceId === null ? {} : { deviceId }),
              }
            : scope === "space"
              ? { kind: scope, location }
              : { kind: scope, hostId };

    return SettingAssignment.parse({ ...record, setting: { packageId, name }, target });
}

/** Map an assignment to its indexed columns. */
export function writeAssignment(record: SettingAssignment): typeof settingAssignment.$inferInsert {
    const { setting, target, ...columns } = record;

    return {
        ...columns,
        ...targetColumns(target),
        source: record.provenance ? sourceKey(record.provenance) : null,
        packageId: setting.packageId,
        name: setting.name,
    };
}

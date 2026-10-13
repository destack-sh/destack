import { Manager } from "@destack/access";
import { type Condition, type Row, TABLE, type Transaction } from "@destack/db";
import { schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { defineMethod, type Method } from "./method.ts";
import type { Call } from "./call.ts";
import { RevisionShape } from "./procedure.ts";

import { isManaging } from "../trait/declarable.ts";
import type { ObjectType } from "../object/object.ts";
import type { TraitTable } from "../object/table.ts";
import type { DeclarableDefinition } from "../trait/declarable.ts";
/** Detach an object from its stack declaration. */
export function detach<const Permission extends string>(
    permission: Permission,
): Method<{ kind: "detach"; permission: Permission; mutates: true }> {
    return defineMethod<{ kind: "detach"; permission: Permission; mutates: true }>({
        kind: "detach",
        permission,
        mutates: true,
        target: true,
        result: "object",
        procedure: (_name, shapes) => ({
            route: { method: "POST", path: "/{id}/detach" },
            input: shapes.target.extend({ ...shapes.replay, ...RevisionShape }),
            output: shapes.row,
        }),
        handler: (call: Call<TraitTable<{ readonly declarable: DeclarableDefinition }>>) =>
            call.update({ detachedAt: call.now }),
        async execute(call) {
            // require an object its declaration still manages
            if (!isManaging(call.target)) {
                throw new CallError("CONFLICT", {
                    message: `${call.object.name} is not managed by a declaration`,
                });
            }

            return this.handler(call);
        },
    });
}

/** The input adopting a record: the declaration taking it over. */
const AdoptInput = schema.object({
    /** The declaration taking over the record. */
    manager: Manager.schema,
});

/** The manager columns of a declared record. */
const ManagerColumns = schema.looseObject({
    /** The installation that applied the declaration. */
    managerInstallationId: schema.string().nullable(),
    /** The declaring package. */
    managerPackageId: schema.string().nullable(),
    /** The declaration's name. */
    managerName: schema.string().nullable(),
});

/** Hand a direct record to a declaration, whose next apply rewrites it with the declared values. */
export function adopt<const Permission extends string>(
    permission: Permission,
): Method<{ kind: "adopt"; permission: Permission; mutates: true }> {
    return defineMethod<{ kind: "adopt"; permission: Permission; mutates: true }>({
        kind: "adopt",
        permission,
        mutates: true,
        target: true,
        result: "object",
        procedure: (_name, shapes) => ({
            route: { method: "POST", path: "/{id}/adopt" },
            input: shapes.target.extend({
                ...shapes.replay,
                ...RevisionShape,
                ...AdoptInput.shape,
            }),
            output: shapes.row,
        }),
        handler: (call: Call<TraitTable<{ readonly declarable: DeclarableDefinition }>>) => {
            const { manager } = AdoptInput.parse(call.input);

            return call.update({ ...Manager.values(manager), detachedAt: null });
        },
        async execute(call) {
            await requireAdoptable(call);

            return this.handler(call);
        },
    });
}

/** Require a direct record, or one detached from the adopting declaration. */
async function requireAdoptable(call: Call): Promise<void> {
    // read the adopting declaration and the target's manager
    const { manager } = AdoptInput.parse(call.input);
    const target: Row | undefined = call.target;
    const managed = target === undefined ? null : Manager.read(ManagerColumns.parse(target));
    const [declared] = await call
        .read(call.object.table)
        .rows(declaredBy(call.object, call.scope, manager));

    // refuse a managed record, a record of another declaration and a declaration with another record
    if (isManaging(target)) {
        throw new CallError("CONFLICT", {
            message: `${call.object.name} is managed by a declaration`,
        });
    } else if (managed !== null && !isSameManager(managed, manager)) {
        throw new CallError("CONFLICT", {
            message: `${call.object.name} is detached from another declaration`,
        });
    } else if (declared !== undefined && declared["id"] !== target?.["id"]) {
        throw new CallError("CONFLICT", {
            message: `declaration ${manager.name} declares another ${call.object.name}`,
        });
    }
}

/** Decide whether two managers refer to the same declaration. */
function isSameManager(left: Manager, right: Manager): boolean {
    return (
        left.installationId === right.installationId &&
        left.packageId === right.packageId &&
        left.name === right.name
    );
}

/** The input applying a declared record: manager and JSON values, null once undeclared. */
const ApplyInput = schema.object({
    /** The declaration managing the record. */
    manager: Manager.schema,
    /** The declared values by column, null to retire the record. */
    values: schema.record(schema.string(), schema.json()).nullable(),
});

/** Apply a stack's declared record where its rows live. */
export const apply: Method<{
    kind: "apply";
    permission: "declare";
    mutates: true;
    internal: true;
}> = defineMethod<{ kind: "apply"; permission: "declare"; mutates: true; internal: true }>({
    kind: "apply",
    permission: "declare",
    mutates: true,
    isInternal: true,
    target: false,
    result: "value",
    procedure: (_name, shapes) => ({
        route: { method: "POST", path: "/apply" },
        input: shapes.scope.extend({ ...shapes.created, ...ApplyInput.shape }),
        output: shapes.row.or(schema.object({})),
    }),
    handler: async (call: Call) => {
        // read the record the manager declared
        const { manager, values } = ApplyInput.parse(call.input);
        const { object, now } = call;
        const transaction = call.requireTransaction();
        const [applied] = await transaction.rows(
            object.table,
            declaredBy(object, call.scope, manager),
        );

        // retire a record no longer declared
        if (values === null) {
            if (applied !== undefined) {
                await retire(transaction, object, applied, now);
            }

            return {};
        }

        // create a missing record under the chosen identifier, or write the values over it as its next revision
        const decoded = object.table[TABLE].decode(values);
        const row =
            applied === undefined
                ? created(object, {
                      id: call.id ?? call.generateId(object),
                      scope: call.scope,
                      manager,
                      values: decoded,
                      now,
                  })
                : rewritten(object, applied, decoded, now);
        await transaction.write({ table: object.table, before: applied ?? null, after: row });

        return row;
    },
});

/** Match the record a manager declared in a scope, leaving out purged ones. */
function declaredBy(object: ObjectType, scope: string, manager: Manager): Condition {
    const isPurgeable = "purgedAt" in object.table[TABLE].columns;

    return {
        scope,
        managerInstallationId: manager.installationId,
        managerPackageId: manager.packageId,
        managerName: manager.name,
        ...(isPurgeable ? { purgedAt: { isNull: true } } : {}),
    };
}

/** Build a declared record under its manager. */
function created(
    object: ObjectType,
    record: {
        readonly id: string;
        readonly scope: string;
        readonly manager: Manager;
        readonly values: Row;
        readonly now: number;
    },
): Row {
    const table = object.table[TABLE];

    return table.complete(
        table.values({
            id: record.id,
            createdAt: record.now,
            updatedAt: record.now,
            ...record.values,
            scope: record.scope,
            ...Manager.values(record.manager),
        }),
    );
}

/** Write declared values as a record's next revision, advancing generation, lifting deletion. */
function rewritten(object: ObjectType, row: Row, values: Row, now: number): Row {
    const table = object.table[TABLE];

    return table.values({
        ...row,
        ...values,
        ...("generation" in table.columns
            ? { generation: schema.number().parse(row["generation"]) + 1 }
            : {}),
        ...("deletionRequestedAt" in table.columns
            ? { deletionRequestedAt: null, deletedBy: null }
            : {}),
        revision: schema.number().parse(row["revision"]) + 1,
        updatedAt: now,
    });
}

/** Retire an undeclared record: request deletion where a trash or controller ends it, or delete. */
async function retire(
    transaction: Transaction,
    object: ObjectType,
    row: Row,
    now: number,
): Promise<void> {
    const table = object.table[TABLE];
    const requested = { ...row, deletionRequestedAt: now, deletedBy: null };
    await transaction.write({
        table: object.table,
        before: row,
        after: "deletionRequestedAt" in table.columns ? table.values(requested) : null,
    });
}

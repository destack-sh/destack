import { ObjectReference } from "@destack/sync";
import { Manager } from "@destack/access";
import { type Condition, type Row, TABLE, type Transaction, Table } from "@destack/db";
import { canonicalize, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { defineMethod, type Method } from "./method.ts";
import type { Call } from "./call.ts";
import { RevisionShape } from "./procedure.ts";

import { declarable } from "../trait/declarable.ts";
import type { ObjectType } from "../object/object.ts";
import { Execution } from "./execution.ts";
import type { DeclarableDefinition } from "../trait/declarable.ts";
import type { TraitTable } from "../object/table.ts";

/** The table of an object type stacks declare. */
type DeclarableTable = TraitTable<{ readonly declarable: DeclarableDefinition }>;

/** Detach an object from its stack declaration. */
export function detach<const Permission extends string>(
    permission: Permission,
): Method<{ kind: "detach"; permission: Permission; mutates: true }> {
    return defineMethod<{
        kind: "detach";
        permission: Permission;
        mutates: true;
        table: DeclarableTable;
    }>({
        kind: "detach",
        permission,
        mutates: true,
        target: "object",
        result: "object",
        procedure: (_name, shapes) => ({
            route: { method: "POST", path: "/{id}/detach" },
            input: shapes.target.extend({ ...shapes.replay, ...RevisionShape }),
            output: shapes.row,
        }),
        handler: (call) => call.update({ detachedAt: call.now }),
        async execute(call, execution) {
            // require an object its declaration still manages
            if (!declarable.isManaging(call.target)) {
                throw new CallError("CONFLICT", {
                    message: `${call.object.name} is not managed by a declaration`,
                });
            }

            return this.handler(call, execution);
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
    return defineMethod<{
        kind: "adopt";
        permission: Permission;
        mutates: true;
        table: DeclarableTable;
    }>({
        kind: "adopt",
        permission,
        mutates: true,
        target: "object",
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
        handler: (call) => {
            const { manager } = AdoptInput.parse(call.input);

            return call.update({ ...Manager.values(manager), detachedAt: null });
        },
        async execute(call, execution) {
            await DeclarableMethod.requireAdoptable(call);

            return this.handler(call, execution);
        },
    });
}

/** The input applying a declared record: manager and JSON values, null once undeclared. */
const ApplyInput = schema.object({
    /** The declaration managing the record. */
    manager: Manager.schema,
    /** The declared values by column, null to retire the record. */
    values: schema.jsonObject().nullable(),
});

/** Apply a stack's declared record where its rows live. */
export const apply: Method<{
    kind: "apply";
    permission: "declare";
    mutates: true;
    internal: true;
    target: "scope";
}> = defineMethod<{
    kind: "apply";
    permission: "declare";
    mutates: true;
    internal: true;
    target: "scope";
}>({
    kind: "apply",
    permission: "declare",
    mutates: true,
    isInternal: true,
    target: "scope",
    result: "value",
    procedure: (_name, shapes) => ({
        route: { method: "POST", path: "/apply" },
        input: shapes.scope.extend({ ...shapes.created, ...ApplyInput.shape }),
        output: shapes.row.or(schema.object({})),
    }),
    handler: async (call: Call, execution: Execution) => {
        // read the record the manager declared
        const { manager, values } = ApplyInput.parse(call.input);
        const { object, now } = call;
        const { transaction } = Execution.store(execution, call);
        const [applied] = await transaction
            .read(object.table)
            .findMany({ where: DeclarableMethod.declaredBy(object, call.scope, manager) });

        // retire a record no longer declared
        if (values === null) {
            if (applied !== undefined) {
                await DeclarableMethod.retire(transaction, object, applied, now);
            }

            return {};
        }

        // refuse changing a column the declaration sets once
        const decoded = Table.decode(object.table, values);
        const fixed = (object.declaration?.immutable ?? []).find(
            (column) =>
                applied !== undefined &&
                column in decoded &&
                canonicalize(applied[column] ?? null) !== canonicalize(decoded[column] ?? null),
        );
        if (fixed !== undefined) {
            throw new CallError("BAD_REQUEST", {
                message: `${object.name} ${fixed} is immutable: declared ${canonicalize(decoded[fixed] ?? null)}, kept ${canonicalize(applied?.[fixed] ?? null)}`,
            });
        }

        // create a missing record under the chosen identifier, or write the values over it as its next revision
        const row =
            applied === undefined
                ? DeclarableMethod.created(object, {
                      id: call.id ?? (await call.generateId(object)),
                      scope: call.scope,
                      manager,
                      values: decoded,
                      now,
                  })
                : DeclarableMethod.rewritten(object, applied, decoded, now);
        const change = { table: object.table, before: applied ?? null, after: row };
        await transaction.write(change);
        object.keepReaders(transaction, change, now);

        return row;
    },
});

/** The declarable methods' handlers: adopting, detaching and applying declared objects. */
const DeclarableMethod = {
    /** Require a direct record, or one detached from the adopting declaration. */
    async requireAdoptable(call: Call): Promise<void> {
        // read the adopting declaration and the target's manager
        const { manager } = AdoptInput.parse(call.input);
        const target: Row | undefined = call.target;
        const managed = target === undefined ? null : Manager.read(ManagerColumns.parse(target));
        const [declared] = await call
            .read(call.object.table)
            .findMany({ where: DeclarableMethod.declaredBy(call.object, call.scope, manager) });

        // refuse a managed record, and a record or declaration of another declaration
        if (declarable.isManaging(target)) {
            throw new CallError("CONFLICT", {
                message: `${call.object.name} is managed by a declaration`,
            });
        } else if (managed !== null && !DeclarableMethod.isSameManager(managed, manager)) {
            throw new CallError("CONFLICT", {
                message: `${call.object.name} is detached from another declaration`,
            });
        } else if (declared !== undefined && declared["id"] !== target?.["id"]) {
            throw new CallError("CONFLICT", {
                message: `declaration ${manager.name} declares another ${call.object.name}`,
            });
        }
    },

    /** Decide whether two managers refer to the same declaration. */
    isSameManager(left: Manager, right: Manager): boolean {
        return (
            left.installationId === right.installationId &&
            left.packageId === right.packageId &&
            left.name === right.name
        );
    },

    /** Match the record a manager declared in a scope, leaving out purged ones. */
    declaredBy(object: ObjectType, scope: string, manager: Manager): Condition {
        const isPurgeable = "purgedAt" in object.table[TABLE].columns;

        return {
            scope,
            managerInstallationId: manager.installationId,
            managerPackageId: manager.packageId,
            managerName: manager.name,
            ...(isPurgeable ? { purgedAt: { isNull: true } } : {}),
        };
    },

    /** Build a declared record under its manager. */
    created(
        object: ObjectType,
        record: {
            readonly id: string;
            readonly scope: string;
            readonly manager: Manager;
            readonly values: Row;
            readonly now: number;
        },
    ): Row {
        return Table.complete(
            object.table,
            Table.values(object.table, {
                id: record.id,
                createdAt: record.now,
                updatedAt: record.now,
                ...record.values,
                scope: record.scope,
                ...Manager.values(record.manager),
            }),
        );
    },

    /** Write declared values as a record's next revision, lifting its deletion. */
    rewritten(object: ObjectType, row: Row, values: Row, now: number): Row {
        return Table.values(object.table, {
            ...row,
            ...values,
            ...("generation" in object.table[TABLE].columns
                ? { generation: schema.number().parse(row["generation"]) + 1 }
                : {}),
            ...("deletedAt" in object.table[TABLE].columns
                ? { deletedAt: null, ...ObjectReference.columns("deletedBy", undefined) }
                : {}),
            revision: schema.number().parse(row["revision"]) + 1,
            updatedAt: now,
        });
    },

    /** Retire an undeclared record: request its deletion, or delete it at once. */
    async retire(
        transaction: Transaction,
        object: ObjectType,
        row: Row,
        now: number,
    ): Promise<void> {
        // request deletion where the table keeps it, else remove the row
        const table = object.table[TABLE];
        const requested = {
            ...row,
            deletedAt: now,
            ...ObjectReference.columns("deletedBy", undefined),
        };
        const change = {
            table: object.table,
            before: row,
            after: "deletedAt" in table.columns ? Table.values(object.table, requested) : null,
        };
        await transaction.write(change);
        object.keepReaders(transaction, change, now);
    },
};

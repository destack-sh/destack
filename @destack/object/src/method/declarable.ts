import { Manager } from "@destack/access";
import {
    and,
    type DatabaseConnection,
    eq,
    type Insert,
    isNull,
    type Row,
    type Select,
    sql,
    TABLE,
    type Table,
} from "@destack/db";
import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
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
                throw new ServiceError("CONFLICT", {
                    message: `${call.object.name} is not managed by a declaration`,
                });
            }

            return this.handler(call);
        },
    });
}

/** The input applying a declared record: its manager and the declared values in their JSON form, null once no longer declared. */
const ApplyInput = schema.object({
    /** The declaration managing the record. */
    manager: Manager.schema,
    /** The declared values by column, null to retire the record. */
    values: schema.record(schema.string(), schema.json()).nullable(),
});

/** Apply the record a stack declares where its rows live: create it under the chosen identifier, write it as its next revision, or retire it once no longer declared. */
export const apply: Method<{
    kind: "apply";
    permission: null;
    mutates: true;
    system: true;
}> = defineMethod<{ kind: "apply"; permission: null; mutates: true; system: true }>({
    kind: "apply",
    permission: null,
    mutates: true,
    isSystem: true,
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
        const { object, database, now } = call;
        const applied = await find(database, object, call.scope, manager);

        // retire a record no longer declared
        if (values === null) {
            if (applied !== undefined) {
                await retire(database, object, applied, now);
            }

            return {};
        }

        // create a missing record under the chosen identifier, or write the values over it as its next revision
        const decoded = object.table[TABLE].decode(values);
        const id =
            applied === undefined
                ? await insert(database, object, {
                      id: call.id ?? object.generateId(),
                      scope: call.scope,
                      manager,
                      values: decoded,
                      now,
                  })
                : await rewrite(database, object, applied, decoded, now);

        return read(database, object, id);
    },
});

/** Find the record a manager declared in a scope, leaving out purged ones. */
async function find(
    database: DatabaseConnection,
    object: ObjectType,
    scope: string,
    manager: Manager,
): Promise<Row | undefined> {
    const table = object.table[TABLE];
    const [row] = await database
        .select()
        .from(object.table)
        .where(
            and(
                eq(table.column("scope"), scope),
                eq(table.column("managerInstallationId"), manager.installationId),
                eq(table.column("managerPackageId"), manager.packageId),
                eq(table.column("managerName"), manager.name),
                "purgedAt" in table.columns ? isNull(table.column("purgedAt")) : undefined,
            ),
        );

    return row;
}

/** Insert a declared record under its manager, answering its identifier. */
async function insert(
    database: DatabaseConnection,
    object: ObjectType,
    record: {
        readonly id: string;
        readonly scope: string;
        readonly manager: Manager;
        readonly values: Partial<Insert<Table>>;
        readonly now: number;
    },
): Promise<string> {
    const table: Table = object.table;
    await database.insert(table).values({
        id: record.id,
        createdAt: record.now,
        updatedAt: record.now,
        ...record.values,
        scope: record.scope,
        ...Manager.values(record.manager),
    });

    return record.id;
}

/** Write declared values over a record as its next revision, advancing its generation and lifting a deletion request, answering its identifier. */
async function rewrite(
    database: DatabaseConnection,
    object: ObjectType,
    row: Row,
    values: Partial<Insert<Table>>,
    now: number,
): Promise<string> {
    // write the values with the next revision and generation, without a deletion request
    const table = object.table[TABLE];
    const columns: Table = object.table;
    const id = schema.string().parse(row["id"]);
    await database
        .update(columns)
        .set({
            ...values,
            ...("generation" in table.columns
                ? { generation: sql`${table.column("generation")} + 1` }
                : {}),
            ...("deletionRequestedAt" in table.columns
                ? { deletionRequestedAt: null, deletedBy: null }
                : {}),
            revision: Number(row["revision"] ?? 0) + 1,
            updatedAt: now,
        })
        .where(eq(table.column("id"), id));

    return id;
}

/** Retire a record no longer declared: request its deletion where a trash or controller finishes it, or delete it. */
async function retire(
    database: DatabaseConnection,
    object: ObjectType,
    row: Row,
    now: number,
): Promise<void> {
    // request the deletion of a deletable record, or delete it
    const table = object.table[TABLE];
    const columns: Table = object.table;
    const id = eq(table.column("id"), schema.string().parse(row["id"]));
    if ("deletionRequestedAt" in table.columns) {
        await database.update(columns).set({ deletionRequestedAt: now, deletedBy: null }).where(id);
    } else {
        await database.delete(columns).where(id);
    }
}

/** Read one applied record by identifier. */
async function read(
    database: DatabaseConnection,
    object: ObjectType,
    id: string,
): Promise<Select<Table>> {
    const [row] = await database
        .select()
        .from(object.table)
        .where(eq(object.table[TABLE].column("id"), id));
    if (row === undefined) {
        throw new TypeError(`the applied ${object.name} ${id} is missing`);
    }

    return row;
}

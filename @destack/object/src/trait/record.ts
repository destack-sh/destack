import {
    AccessError,
    accessRelationship,
    isPrincipal,
    subjectKey,
    type Access,
    type Creation,
    type GrantReader,
} from "@destack/access";
import {
    and,
    type Column,
    decodeRow,
    encodeRow,
    eq,
    identifier,
    integer,
    inArray,
    isNull,
    json,
    sql,
    TABLE,
    text,
    type DatabaseConnection,
    type Insert,
    type Select,
    type SQL,
    type Table,
} from "@destack/db";
import { type Scalar } from "@destack/db/query";
import { type Expression } from "@destack/schema/expression";
import { changesThroughLog, Dataflow, View, type ObjectReference } from "@destack/sync";
import { canonicalize } from "@destack/schema/json";
import { type Identifier, schema, type Version } from "@destack/schema";
import { DatabaseError } from "@destack/db/error";
import { conceal, ServiceError } from "@destack/service/error";
import { Page, page } from "@destack/service/page";
import { v7 } from "uuid";
import { Call } from "../method/call.ts";
import { Step } from "../method/step.ts";
import type * as sync from "@destack/sync";
import { defineMethod, type Method } from "../method/method.ts";
import {
    Empty,
    PageShape,
    RevisionShape,
    type FieldShape,
    type WrittenShape,
    type ParentShape,
    type Procedure,
    type ReplayShape,
    type RevisionField,
    type RowSchema,
    type ScopeShape,
    type TargetShape,
} from "../method/procedure.ts";
import { kebabCase } from "../object/name.ts";
import { Listing } from "../query/listing.ts";
import { ListedShape, QueryShape, ViewShape, type ObjectInclude } from "../replica/replica.ts";
import type { ObjectType } from "../object/object.ts";
import { versioned } from "./versioned.ts";
import { Manager } from "./declarable.ts";
import type { Trait } from "./trait.ts";

/** User-defined labels indexed by name. */
export const TagMap = schema.record(schema.string().min(1).max(128), schema.string().max(256));
/** User-defined labels indexed by name. */
export type TagMap = schema.Infer<typeof TagMap>;

/** Declare a label map with an empty database default. */
export function tags(name = "tags") {
    return json(name, TagMap)
        .notNull()
        .default(sql`'{}'`);
}

/** Declare the columns every record holds. */
export function recordColumns<const Prefix extends string>(prefix: Prefix) {
    return {
        /** The immutable record identifier. */
        id: identifier("id", prefix).primaryKey(),
        /** Creation time in UTC epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** Last modification time in UTC epoch milliseconds. */
        updatedAt: integer("updated_at").notNull(),
        /** The revision used by conditional updates. */
        revision: integer("revision").notNull().default(1),
        /** User-defined labels. */
        tags: tags(),
    };
}

/** The columns every record holds. */
export type RecordBuilderMap<Prefix extends string = string> = ReturnType<
    typeof recordColumns<Prefix>
>;

/** Declare the column naming an ephemeral object's writing client. */
export function clientColumns() {
    return {
        /** The identifier of the client that wrote the object. */
        client: text("client").notNull(),
    };
}

/** The column naming an ephemeral object's writing client. */
export type ClientBuilderMap<Storage> = Storage extends "ephemeral"
    ? ReturnType<typeof clientColumns>
    : {};

/** The record trait every object takes. */
export const record: Trait<true> = {
    options: () => true,
    columns: (_options, object) => ({
        ...recordColumns(object.identity),
        ...(object.storage === "ephemeral" ? clientColumns() : {}),
    }),
    constraints: () => [],
    // cascade durable objects' relationships
    table: (_options, object) =>
        object.storage === "ephemeral"
            ? {}
            : {
                  dependents: [
                      {
                          from: () => accessRelationship,
                          key: "objectId",
                          where: { packageId: object.packageId, type: object.name },
                          onDelete: "cascade",
                      },
                  ],
              },
    methods: () => ({}),
};

/** Read one object. */
export function get<const Permission extends string>(
    permission: Permission,
): Method<"get", Permission, never, never, false> {
    return defineMethod<Method<"get", Permission, never, never, false>>({
        kind: "get",
        permission,
        mutates: false,
        target: true,
        result: "object",
        procedure: (_name, shapes) => ({
            route: { method: "GET", path: "/{id}" },
            input: shapes.target.extend(ViewShape),
            output: shapes.row,
        }),
        effect: async (call) => call.target,
    });
}

/** Read a page of a scope's objects or aggregate groups. */
export function list<const Permission extends string>(
    permission: Permission,
): Method<"list", Permission, never, never, false> {
    return defineMethod<Method<"list", Permission, never, never, false>>({
        kind: "list",
        permission,
        mutates: false,
        target: false,
        result: "page",
        procedure: (_name, shapes) => ({
            route: { method: "POST", path: "/query" },
            input: shapes.scope.extend({ ...QueryShape, ...PageShape, ...ViewShape }),
            output: page(shapes.row).extend(ListedShape),
        }),
        effect: listObjects,
    });
}

/** Create an object from the fields the caller writes. */
export function create<
    const Permission extends string | null,
    const Fields extends string = string,
    Input extends schema.Schema = never,
>(
    permission: Permission,
    options: {
        /** Whether only the system creates, with no permission. */
        readonly isSystem?: true;
        readonly fields?: readonly Fields[];
        readonly input?: Input;
        /** The relation the calling principal takes to the new object. */
        readonly creator?: string;
        /** The new object's first relationships and a new scope's owner. */
        readonly creation?: (call: Call, object: ObjectReference) => Creation | Promise<Creation>;
        /** Whether clients predict the creation. */
        readonly isPredicted?: false;
        /** The input fields each release computes from an earlier call's input, by the release introducing them. */
        readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
    } = {},
): Method<"create", Permission, NoInfer<Input>, never, true, NoInfer<Fields>> {
    // resolve the creation
    const { creator, creation: declaredCreation, ...declared } = options;
    if (creator !== undefined && declaredCreation !== undefined) {
        throw new TypeError("a creation names a creator relation or a creation, not both");
    }
    const creation = creator === undefined ? declaredCreation : relateCreator(creator);

    return defineMethod<Method<"create", Permission, Input, never, true, Fields>>({
        kind: "create",
        permission,
        mutates: true,
        ...declared,
        target: false,
        result: "object",
        procedure: (_name, shapes) => ({
            route: { method: "POST", path: "" },
            input: shapes.scope.extend({
                ...shapes.created,
                ...shapes.parent,
                ...shapes.replay,
                ...shapes.written(options.fields, false),
                ...fields(options.input),
            }),
            output: shapes.row,
        }),
        effect: createObject,
        authorize: async (call) => {
            // require the receiving parent and the create permission of a caller
            if (call.method.permission === null) {
                return;
            }
            const reader = await call.authorization!.reader(call.scope);
            const parent = call.object.parent && call.parent();
            if (parent) {
                await call.requireReceiving(parent, reader);
            }
            await requireCreatable(call, reader, creation);
        },
        inverse: (step) => {
            // delete the created object
            const call = Step.call(step, "delete", Step.target(step, step.after?.id));

            return call === undefined ? undefined : [call];
        },
        async execute(call) {
            // predict under the client's identifier
            if (call.isPredicted) {
                if (call.id === undefined) {
                    throw new TypeError(
                        `predicted ${call.object.name} creations need an identifier`,
                    );
                }

                return this.effect(call);
            }

            // insert under an unused identifier
            const authorization = call.served();
            const { created, row } = await insertCreated(call, (inserted) => this.effect(inserted));

            // record access and require guarded writes
            const id = Call.resultId(row)!;
            const object = call.object.reference(call.scope, id);
            const isScope = call.object.policy.definition.scope === true;
            if (isScope || creation !== undefined) {
                await authorization.create(object, (await creation?.(created, object)) ?? {});
            }
            await authorization.requireWritable(created, id);

            return row;
        },
    });
}

/** Update an object's written fields. */
export function update<
    const Permission extends string | null,
    const Fields extends string = string,
    Input extends schema.Schema = never,
>(
    permission: Permission,
    options: {
        /** Whether only the system updates, with no permission. */
        readonly isSystem?: true;
        readonly fields?: readonly Fields[];
        readonly input?: Input;
        /** Whether clients predict the update. */
        readonly isPredicted?: false;
        /** The input fields each release computes from an earlier call's input, by the release introducing them. */
        readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
    } = {},
): Method<"update", Permission, NoInfer<Input>, never, true, NoInfer<Fields>> {
    return defineMethod<Method<"update", Permission, Input, never, true, Fields>>({
        kind: "update",
        permission,
        mutates: true,
        ...options,
        target: true,
        result: "object",
        procedure: (_name, shapes) => ({
            route: { method: "PATCH", path: "/{id}" },
            input: shapes.target.extend({
                ...shapes.replay,
                ...RevisionShape,
                ...shapes.written(options.fields, true),
                ...fields(options.input),
            }),
            output: shapes.row,
        }),
        effect: (call) => call.revise(decodeRow(call.object.table as Table, call.input)),
        inverse: (step) => {
            // restore fields unchanged since
            const current = step.current;
            if (current === undefined) {
                return undefined;
            }
            const restored = Object.fromEntries(
                step.object.written
                    .filter((name) => Object.hasOwn(step.input, name))
                    .filter((name) => Step.same(current[name], step.after?.[name]))
                    .map((name) => [name, step.before?.[name] ?? null]),
            );
            if (Object.keys(restored).length === 0) {
                return undefined;
            }

            return [
                Step.record(step, step.name, { ...Step.target(step, step.input.id), ...restored }),
            ];
        },
        async execute(call) {
            // refuse managed objects and unwritable fields
            requireUnmanaged(call);
            if (!call.isPredicted) {
                await call.served().requireWritable(call, call.id!);
            }

            return this.effect(call);
        },
    });
}

/** Update every matching object the caller may change, returning the count. */
export function updateMany<
    const Permission extends string,
    const Fields extends string = string,
    const Match extends string = string,
>(
    permission: Permission,
    options: {
        /** The fields the call writes on each matched object. */
        readonly fields: readonly Fields[];
        /** The fields the call matches objects by. */
        readonly match: readonly Match[];
        /** The input fields each release computes from an earlier call's input, by the release introducing them. */
        readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
    },
): Method<"updateMany", Permission, never, never, true, NoInfer<Fields>> {
    return defineMethod<Method<"updateMany", Permission, never, never, true, Fields>>({
        kind: "updateMany",
        permission,
        mutates: true,
        ...(options.convert === undefined ? {} : { convert: options.convert }),
        target: false,
        result: "value",
        procedure: (_name, shapes) => ({
            route: { method: "PATCH", path: "" },
            input: shapes.scope.extend({
                ...shapes.replay,
                where: schema.object(shapes.written(options.match, true)),
                ...shapes.written(options.fields, true),
            }),
            output: schema.object({ count: schema.number().int().nonnegative() }),
        }),
        validate: (object) => {
            // refuse tracked, declarable, versioned objects and guarded fields
            if (
                object.tracked !== undefined ||
                object.declarationSchema !== undefined ||
                object.versioned
            ) {
                throw new TypeError(
                    `object ${object.name} keeps a record of each change, which many updates at once skip`,
                );
            }
            const guarded = options.fields.find(
                (name) => object.fields[name]?.access?.write !== undefined,
            );
            if (guarded !== undefined) {
                throw new TypeError(
                    `object ${object.name} guards field ${guarded}, which many updates at once skip`,
                );
            }
        },
        effect: async (call) => {
            // decode values and matches
            const table = call.object.table as Table & Record<string, Column>;
            const input = call.input as Record<string, unknown> & {
                readonly where: Record<string, unknown>;
            };
            const values = decodeRow(
                table,
                Object.fromEntries(
                    options.fields.flatMap((name) =>
                        Object.hasOwn(input, name) ? [[name, input[name]]] : [],
                    ),
                ),
            );
            const matched = decodeRow(table, input.where);
            const hasGeneration = Object.hasOwn(table[TABLE].columns, "generation");

            // match by the given values
            const matching = and(
                call.object.inScope(call.scope),
                ...Object.entries(matched).map(([name, value]) =>
                    value === null ? isNull(table[name]!) : eq(table[name]!, value),
                ),
            );

            // restrict served calls to permitted objects, then update
            let held: SQL | undefined;
            if (!call.isPredicted) {
                const served = call.served();
                const permission = call.object.permission(call.method.permission!);
                const listable = served.listable(call.object, permission);
                if (listable === "memory") {
                    const rows = (await call.database
                        .select()
                        .from(table)
                        .where(matching)) as Record<string, unknown>[];
                    const kept = await served.keep(rows, permission);
                    held = inArray(
                        table.id!,
                        kept.map((row) => row.id),
                    );
                } else {
                    held = listable;
                }
            }
            const changed = (await call.database
                .update(table)
                .set({
                    ...values,
                    revision: sql`${table.revision!} + 1`,
                    ...(hasGeneration ? { generation: sql`${table.generation!} + 1` } : {}),
                    updatedAt: call.now,
                } as Partial<Insert<Table>>)
                .where(and(matching, held))
                .returning({ id: table.id! })) as unknown[];

            return { count: changed.length };
        },
    });
}

/** Delete an object. */
export function remove<const Permission extends string | null>(
    permission: Permission,
    options: {
        /** Whether only the system deletes, with no permission. */
        readonly isSystem?: true;
    } = {},
): Method<"delete", Permission, never, never, true> {
    return defineMethod<Method<"delete", Permission, never, never, true>>({
        kind: "delete",
        permission,
        mutates: true,
        ...options,
        target: true,
        result: "value",
        procedure: (_name, shapes) => ({
            route: { method: "DELETE", path: "/{id}" },
            input: shapes.target.extend({ ...shapes.replay, ...RevisionShape }),
            output: Empty,
        }),
        effect: deleteObject,
        inverse: (step) => {
            // restore from the trash
            const call =
                step.object.recoverable === undefined
                    ? undefined
                    : Step.call(step, "restore", Step.target(step, step.input.id));

            return call === undefined ? undefined : [call];
        },
        async execute(call) {
            requireUnmanaged(call);

            return this.effect(call);
        },
    });
}

/** Declare a custom method on one object, mutating by default. */
export function custom<
    const Permission extends string | null,
    Input extends schema.Schema = never,
    Output extends schema.Schema = never,
    const Mutates extends boolean = true,
>(definition: {
    /** The permission the caller needs on the target, null for none. */
    readonly permission: Permission;
    /** Whether only the system calls the method. */
    readonly isSystem?: true;
    /** The caller-supplied fields. */
    readonly input?: Input;
    /** The result, the updated object when absent. */
    readonly output?: Output;
    /** Whether the method changes state. */
    readonly mutates?: Mutates;
    /** Whether each reading call is audited. */
    readonly audited?: true;
    /** The fields of the result an audited read's event names. */
    readonly audit?: { readonly details: schema.Object<Record<string, schema.Schema>> };
    /** The inverse method name, or a function deriving inverse calls. */
    readonly inverse?: string | ((step: Step) => readonly sync.Call[] | undefined);
    /** The input fields each release computes from an earlier call's input, by the release introducing them. */
    readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
}): Method<"custom", Permission, Input, Output, NoInfer<Mutates>> {
    const mutates = (definition.mutates ?? true) as Mutates;
    const { inverse, ...declared } = definition;

    return defineMethod<Method<"custom", Permission, Input, Output, Mutates>>({
        kind: "custom",
        ...declared,
        ...(inverse === undefined
            ? {}
            : {
                  inverse:
                      typeof inverse === "string"
                          ? (step: Step) => [Step.record(step, inverse, step.input)]
                          : inverse,
              }),
        mutates,
        isPredicted: false,
        target: true,
        result: definition.output === undefined ? "object" : "value",
        procedure: (name, shapes) => ({
            route: { method: "POST", path: `/{id}/${kebabCase(name)}` },
            input: shapes.target.extend({
                ...(mutates ? shapes.replay : {}),
                ...fields(definition.input),
            }),
            output: definition.output ?? shapes.row,
        }),
        effect: missingHandler,
    });
}

/** Read a method's own input fields. */
function fields(input: schema.Schema | undefined): Record<string, schema.Schema> {
    return (input as schema.Object<Record<string, schema.Schema>> | undefined)?.shape ?? {};
}

/** Insert a created object. */
async function createObject(call: Call): Promise<Record<string, unknown>> {
    const table = call.object.table as Table & Record<string, never>;
    const [row] = (await call.database
        .insert(table)
        .values((await createdValues(call)) as Insert<Table>)
        .returning()) as Record<string, unknown>[];

    return row!;
}

/** Build the columns a created object is inserted with. */
async function createdValues(call: Call): Promise<Record<string, unknown>> {
    // require a caller of the declared kinds
    const { object, caller } = call;
    const isSystem = call.method.isSystem === true;
    const callers = Object.entries(object.fields).filter(
        ([, declared]) => declared.isCaller && !isSystem,
    );
    for (const [, declared] of callers) {
        const kinds = declared.principals;
        if (
            caller === undefined ||
            !(kinds === undefined ? isPrincipal(caller) : kinds.some((kind) => kind.is(caller)))
        ) {
            const named = kinds?.map((kind) => kind.name).join(" or ") ?? "principal";
            throw new ServiceError("FORBIDDEN", {
                message: `${object.name} is created by a ${named}`,
            });
        }
    }

    // number versions within their parent
    const version = object.versioned
        ? await versioned.next(object, call.parent()!.id, call.database)
        : undefined;

    // require the writing client
    if (object.storage === "ephemeral" && call.client === undefined) {
        throw new TypeError(`ephemeral ${object.name} objects are created by a client`);
    }

    // assemble the columns
    const table = object.table as Table & Record<string, never>;

    return {
        ...decodeRow(table, call.input),
        ...(object.parent === undefined ? {} : call.parentColumns()),
        id: call.id,
        scope: call.scope,
        ...(version === undefined ? {} : { number: version }),
        ...(object.storage === "ephemeral" ? { client: call.client } : {}),
        ...Object.fromEntries(
            callers.map(([name, declared]) => [
                name,
                declared.type === "subject" ? subjectKey(caller!) : caller!.id,
            ]),
        ),
        createdAt: call.now,
        updatedAt: call.now,
    };
}

/** Insert a created object under an unused identifier within a savepoint. */
async function insertCreated(
    call: Call,
    effect: (created: Call) => Promise<unknown>,
): Promise<{ readonly created: Call; readonly row: unknown }> {
    // insert a prediction directly
    if (call.isPredicted) {
        const created = call.with({ id: await createdId(call) });

        return { created, row: await effect(created) };
    }

    // insert within a savepoint
    try {
        return await call.database.transaction(async (transaction) => {
            // insert under an unused identifier
            const inserting = call.with({ database: transaction });
            const created = inserting.with({ id: await createdId(inserting) });
            const row = await effect(created);

            return { created: created.with({ database: call.database }), row };
        });
    } catch (error) {
        // report conflicts only to admitted callers
        const isConflict =
            (error instanceof ServiceError && error.code === "CONFLICT") ||
            (error instanceof DatabaseError &&
                (error.code === "DUPLICATE" || error.code === "BROKEN_REFERENCE"));
        if (isConflict && call.method.permission !== null) {
            await requireCreatable(call);
        }
        throw error;
    }
}

/** Require the create permission on the object as written. */
async function requireCreatable(
    call: Call,
    reader?: GrantReader,
    creation?: (call: Call, object: ObjectReference) => Creation | Promise<Creation>,
): Promise<void> {
    // build the row with column defaults
    const table = call.object.table as Table;
    const columns = table[TABLE].columns;
    const values = await createdValues(
        call.with({ id: call.id ?? `${call.object.identity}-${v7()}` }),
    );
    const defaults = Object.entries(columns).flatMap(([name, column]) => {
        const value = column.definition.default;
        const isExpression = typeof value === "object" && value !== null && "getSQL" in value;

        return Object.hasOwn(values, name) || value === undefined || isExpression
            ? []
            : [[name, value]];
    });
    const row = { ...Object.fromEntries(defaults), ...values };

    // check the permission on the row
    const authorization = call.authorization!;
    const { authorizer, snapshot } = authorization;
    const grants = reader ?? (await authorization.reader(call.scope));
    const created = call.object.reference(call.scope, String(row.id));
    const initial = (await creation?.(call.with({ id: String(row.id) }), created)) ?? {};
    grants.creating(created, authorizer.initialRelationships(created, initial, call.now));
    const permission = call.object.permission(call.method.permission!);
    const admits = async (access: Access) =>
        (await authorizer.checkRows(snapshot, permission, access, [row], grants)).held.has(0);
    const access = await authorization.in(call.scope);
    if (await admits(access)) {
        return;
    }

    // challenge for step-up authentication
    const stepUp = await authorizer.challenge(snapshot, permission, access, admits);
    if (stepUp !== undefined) {
        throw new AccessError(
            "INSUFFICIENT_AUTHENTICATION",
            "authenticate again at the required assurance",
            { stepUp },
        );
    }

    // check whether the caller reads the object
    const reading = call.object.reading;
    const isReadable =
        reading !== undefined &&
        (await authorizer.checkRows(snapshot, reading, access, [row])).held.has(0);

    // refuse a reader, and hide the object from others
    const denial = new ServiceError("FORBIDDEN", {
        message: `permission denied: ${permission.name}`,
    });
    throw isReadable ? denial : conceal(denial, `${call.object.name} not found`);
}

/** Delete an object, or request deletion when a trash or controller finishes it. */
async function deleteObject(call: Call): Promise<Record<string, never>> {
    // refuse deleting an object twice
    const { object } = call;
    const table = object.table as Table & Record<string, never>;
    const target = call.target as Record<string, unknown>;
    if (target.deletionRequestedAt !== undefined && target.deletionRequestedAt !== null) {
        throw new ServiceError("CONFLICT", { message: `${object.name} is already deleted` });
    }

    // request deletion when a trash or controller finishes it
    if (Object.hasOwn(table[TABLE].columns, "deletionRequestedAt")) {
        await call.revise({ deletionRequestedAt: call.now });

        return {};
    }

    // delete at the loaded revision
    await call.remove();

    return {};
}

/** Accept a caller's unused identifier for a created object, or mint one. */
async function createdId(call: Call): Promise<string> {
    // mint a missing identifier
    const { object } = call;
    const chosen = call.id as Identifier<string> | undefined;
    if (chosen === undefined) {
        return `${object.identity}-${v7()}`;
    }

    // check rows and relationships for the identifier
    const table = object.table as Table & Record<string, never>;
    const { packageId, name } = object.policy.definition;
    const related = sql`EXISTS (
        SELECT 1 FROM ${accessRelationship}
        WHERE ${and(
            eq(accessRelationship.packageId, packageId),
            eq(accessRelationship.type, name),
            eq(accessRelationship.objectId, chosen),
        )}
    )`;
    const held = sql`EXISTS (SELECT 1 FROM ${table} WHERE ${eq(table.id, chosen)})`;
    const isTaken = async (database: DatabaseConnection, exists: SQL) => {
        const [row] = await database.execute<{ isTaken: number | boolean }>(
            sql`SELECT ${exists} AS "isTaken"`,
        );

        return Boolean(row!.isTaken);
    };

    // refuse a taken identifier
    const taken =
        object.storage === "durable"
            ? await isTaken(call.database, sql`(${held} OR ${related})`)
            : (await isTaken(call.database, held)) ||
              (await isTaken(call.authorization!.database, related));
    if (taken) {
        throw new ServiceError("CONFLICT", { message: `${object.name} identifier is taken` });
    }

    return chosen;
}

/** Read a page of a query's rows or its aggregate groups. */
async function listObjects(call: Call) {
    // bind the cursor to the query
    const {
        cursor,
        limit,
        at: _at,
        branch: _branch,
        ...shape
    } = call.input as ObjectInclude & {
        readonly cursor?: string;
        readonly limit?: number;
        readonly at?: unknown;
        readonly branch?: unknown;
    };
    const listing = new Page(
        { ...(cursor === undefined ? {} : { cursor }), ...(limit === undefined ? {} : { limit }) },
        [call.object.name, call.scope, canonicalize(shape)],
        schema.record(schema.string(), schema.json()),
    );

    // compile the query with one extra row
    const compiled = call.object.query(
        shape.aggregate === undefined ? { ...shape, limit: listing.limit + 1 } : shape,
        call.objects,
    );
    const dataflow = new Dataflow(
        { list: { ...compiled, scopes: call.object.scopesOf(call.chain) } },
        {
            audience: new Listing(call),
            database: call.database,
            changesThrough: changesThroughLog(call.database),
            isMaterialized: true,
        },
    );
    const node = dataflow.roots[0]!;

    // read groups at the latest position, or rows
    const position = call.snapshot?.position;
    const after = listing.after === undefined ? undefined : decodeRow(node.table, listing.after);
    await dataflow.fill(await View.of(call.database, call.snapshot), after);
    if (node.aggregate !== undefined && position !== undefined) {
        throw new ServiceError("BAD_REQUEST", {
            message: "aggregates read the latest position, whose access decides them",
        });
    } else if (node.aggregate !== undefined) {
        return { items: [], cursor: null, groups: await dataflow.read("list") };
    }

    // keep, at a position, the rows the caller could also read then
    const read = await dataflow.read("list");
    const rows =
        position === undefined
            ? read
            : await call
                  .served()
                  .keepAt(
                      read,
                      call.object.permission(call.method.permission!),
                      call.snapshot!,
                      call.scope,
                  );
    const ordered = node.order.flatMap((key) => node.columnsOf(key.column));
    const listed = listing.result(rows, (row) =>
        encodeRow(node.table, Object.fromEntries(ordered.map((name) => [name, row[name]]))),
    );

    // split includes and computed values
    const names = node.children
        .filter((child) => child.kind === "include")
        .map((child) => child.name.slice(node.name.length + 1));
    const computed = Object.keys(node.computed);
    const values = Object.fromEntries(
        listed.items.map((row) => [
            String(row.id),
            Object.fromEntries(
                computed.map((name) => [name, node.json(name, row[name]) as Scalar]),
            ),
        ]),
    );
    const included = Object.fromEntries(
        names.map((name) => [
            name,
            Object.fromEntries(
                listed.items.map((row) => [String(row.id), schema.json().parse(row[name])]),
            ),
        ]),
    );
    const items = listed.items.map((row) =>
        Object.fromEntries(
            Object.entries(row).filter(
                ([name]) => !names.includes(name) && !computed.includes(name),
            ),
        ),
    );

    return {
        ...listed,
        items,
        ...(names.length === 0 ? {} : { included }),
        ...(computed.length === 0 ? {} : { computed: values }),
    };
}

/** Refuse standard changes to managed objects. */
function requireUnmanaged(call: Call): void {
    if (Manager.isManaging(call.target)) {
        throw new ServiceError("MANAGED", {
            status: 409,
            message: `${call.object.name} is managed by its stack; detach it before changing it`,
        });
    }
}

/** Reject a method without a handler. */
function missingHandler(call: Call): never {
    throw new TypeError(`object ${call.object.name} implements no handler for ${call.name}`);
}

/** The procedure each record method derives. */
export type RecordProcedures<Object extends ObjectType, Declared extends Method> = {
    get: Procedure<schema.Object<TargetShape<Object> & typeof ViewShape>, RowSchema<Object>>;
    list: Procedure<
        schema.Object<ScopeShape<Object> & typeof QueryShape & typeof PageShape & typeof ViewShape>,
        schema.Object<ReturnType<typeof page<RowSchema<Object>>>["shape"] & typeof ListedShape>
    >;
    create: Procedure<
        schema.Object<
            ScopeShape<Object> &
                CreatedShape<Object> &
                ParentShape<Object, false> &
                ReplayShape<Object> &
                WrittenShape<Object, Declared, false> &
                FieldShape<Declared>
        >,
        RowSchema<Object>
    >;
    update: Procedure<
        schema.Object<
            TargetShape<Object> &
                ReplayShape<Object> &
                RevisionField &
                WrittenShape<Object, Declared, true> &
                FieldShape<Declared>
        >,
        RowSchema<Object>
    >;
    updateMany: Procedure<
        schema.Object<
            ScopeShape<Object> &
                ReplayShape<Object> & { where: schema.Object<MatchShape<Object>> } & WrittenShape<
                    Object,
                    Declared,
                    true
                >
        >,
        schema.Object<{ count: schema.Schema<number> }>
    >;
    delete: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & RevisionField>,
        schema.Object<{}>
    >;
    custom: Procedure<
        schema.Object<
            TargetShape<Object> &
                (Declared["mutates"] extends false ? {} : ReplayShape<Object>) &
                FieldShape<Declared>
        >,
        [NonNullable<Declared["output"]>] extends [never]
            ? RowSchema<Object>
            : NonNullable<Declared["output"]>
    >;
};

/** The values a call matches objects by. */
type MatchShape<Object extends ObjectType> = {
    [Name in keyof Select<Object["table"]> & string]: schema.Optional<
        schema.Schema<Select<Object["table"]>[Name]>
    >;
};

/** The identifier a caller may choose for a created object. */
type CreatedShape<Object extends ObjectType> = {
    id: schema.Optional<
        schema.Schema<Select<Object["table"]>["id" & keyof Select<Object["table"]>]>
    >;
};

/** Relate the calling principal to a new object. */
function relateCreator(relation: string): (call: Call, object: ObjectReference) => Creation {
    return (call, object) => {
        // require a creator
        if (call.caller === undefined) {
            throw new ServiceError("FORBIDDEN", { message: `${call.object.name} needs a creator` });
        }
        const isScope = call.object.policy.definition.scope === true;

        return {
            relationships: [{ relation, subject: call.caller }],
            ...(isScope ? { owner: { ...object, relation } } : {}),
        };
    };
}

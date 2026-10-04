import {
    AccessError,
    accessRelationship,
    isPrincipal,
    type Access,
    type Creation,
    type GrantReader,
} from "@destack/access";
import {
    and,
    eq,
    identifier,
    integer,
    inArray,
    isNull,
    json,
    sql,
    TABLE,
    text,
    type InsertValue,
    type JsonOf,
    type Select,
    type SQL,
    type Table,
    type Expression,
    ColumnValue,
    DatabaseError,
    isSQLWrapper,
    type Row,
} from "@destack/db";
import { validated } from "../field/field.ts";
import { aligned, canonicalize, schema, type JsonObject, type Version } from "@destack/schema";
import { changesThroughLog, Dataflow, View, type ObjectReference, Subject } from "@destack/sync";
import { conceal, ServiceError } from "@destack/service/error";
import { Page, page } from "@destack/service/page";
import { Call, type Bivariant } from "../method/call.ts";
import { Step } from "../method/step.ts";
import type * as sync from "@destack/sync";
import {
    defineMethod,
    type Method,
    type MethodBuilder,
    type MethodDefinition,
} from "../method/method.ts";
import {
    Empty,
    PageShape,
    type MethodProcedure,
    type ObjectSchema,
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
import { nest } from "../query/item.ts";
import { ListShape, QueryShape, ViewShape } from "../replica/replica.ts";
import type { ObjectType } from "../object/object.ts";
import type { ObjectTable } from "../object/table.ts";
import type { RecoverableTable } from "./recoverable.ts";
import { versioned } from "./versioned.ts";
import { Manager } from "./declarable.ts";
import type { Trait } from "./trait.ts";

/** The count of objects a bulk update changed, after Prisma's batch payload. */
export const BatchPayload = schema.object({ count: schema.number().int().nonnegative() });

/** The query, page and view of a list's input, beside its scope fields. */
const LIST_INPUT = schema.looseObject({ ...QueryShape, ...PageShape, ...ViewShape });

/** The field values a bulk update matches objects by, by field name. */
const MATCH = schema.record(schema.string(), schema.json());

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

/** Declare the columns every record has. */
export function recordColumns<const Prefix extends string>(prefix: Prefix) {
    return {
        /** The immutable record identifier. */
        id: identifier("id", prefix).primaryKey(),
        /** Creation time in UTC epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** The subject whose call created the record, by subject key, absent for the platform's own writes. */
        createdBy: text("created_by"),
        /** Last modification time in UTC epoch milliseconds. */
        updatedAt: integer("updated_at").notNull(),
        /** The subject whose call last modified the record, by subject key, absent for the platform's own writes. */
        updatedBy: text("updated_by"),
        /** The revision used by conditional updates. */
        revision: integer("revision").notNull().default(1),
        /** User-defined labels. */
        tags: tags(),
    };
}

/** Declare the identifier column of objects named by their natural key. */
export function keyColumn<Key extends schema.Schema<string>>(key: Key) {
    return validated("id", key).primaryKey();
}

/** The columns every record has, its identifier the natural key when the objects declare one. */
export type RecordBuilderMap<Prefix extends string = string, Key = undefined> =
    Key extends schema.Schema<string>
        ? Omit<ReturnType<typeof recordColumns<Prefix>>, "id"> & {
              readonly id: ReturnType<typeof keyColumn<Key>>;
          }
        : ReturnType<typeof recordColumns<Prefix>>;

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
        ...(object.key === undefined ? {} : { id: keyColumn(object.key) }),
        ...(object.storage === "ephemeral" ? clientColumns() : {}),
    }),
    constraints: () => [],
    // cascade durable objects' relationships
    table: (_options, object) =>
        object.storage !== "durable"
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
export function get<const Permission extends string, Definition extends Table = Table>(
    this: MethodBuilder<Definition>,
    permission: Permission,
): Method<{ kind: "get"; permission: Permission; mutates: false; table: Definition }> {
    return defineMethod<{ kind: "get"; permission: Permission; mutates: false; table: Definition }>(
        {
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
            handler: async (call) => call.target,
        },
    );
}

/** Read a page of a scope's objects or aggregate groups. */
export function list<const Permission extends string, Definition extends Table = Table>(
    this: MethodBuilder<Definition>,
    permission: Permission,
): Method<{ kind: "list"; permission: Permission; mutates: false; table: Definition }> {
    return defineMethod<{
        kind: "list";
        permission: Permission;
        mutates: false;
        table: Definition;
    }>({
        kind: "list",
        permission,
        mutates: false,
        target: false,
        result: "page",
        procedure: (_name, shapes) => ({
            route: { method: "POST", path: "/query" },
            input: shapes.scope.extend({ ...QueryShape, ...PageShape, ...ViewShape }),
            output: page(shapes.row).extend(ListShape),
        }),
        handler: listObjects,
    });
}

/** Derive a new object's first relationships and a new scope's owner, taking any call of the creation as a method would. */
export type CreationOf<Definition extends Table, Input extends schema.Schema> = Bivariant<
    (call: Call<Definition, Input>, object: ObjectReference) => Creation | Promise<Creation>
>;

/** How a creation method creates: its written fields, input, creator and prediction. */
export interface CreateOptions<
    Fields extends string,
    Input extends schema.JsonObject,
    Prepared extends schema.Schema,
    System extends boolean,
    Definition extends Table = Table,
> {
    /** Whether only the system creates, with no permission, naming the caller fields itself. */
    readonly isSystem?: System;
    /** The fields the caller writes, every written field when absent. */
    readonly fields?: readonly Fields[];
    /** The caller-supplied values beyond fields. */
    readonly input?: Input;
    /** The external work the creation prepares before its transaction. */
    readonly prepared?: Prepared;
    /** The relation the calling principal takes to the new object. */
    readonly creator?: string;
    /** The new object's first relationships and a new scope's owner. */
    readonly creation?: CreationOf<Definition, NoInfer<Input>>;
    /** Whether clients predict the creation. */
    readonly isPredicted?: false;
    /** The input fields each release computes from an earlier call's input, by the release introducing them. */
    readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
}

/** How an update method updates: its written fields, input and prediction. */
export interface UpdateOptions<
    Fields extends string,
    Input extends schema.JsonObject,
    Prepared extends schema.Schema,
> {
    /** Whether only the system updates, with no permission. */
    readonly isSystem?: true;
    /** The fields the caller writes, every written field when absent. */
    readonly fields?: readonly Fields[];
    /** The caller-supplied values beyond fields. */
    readonly input?: Input;
    /** The external work the update prepares before its transaction. */
    readonly prepared?: Prepared;
    /** Whether clients predict the update. */
    readonly isPredicted?: false;
    /** The input fields each release computes from an earlier call's input, by the release introducing them. */
    readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
}

/** How a bulk update matches and writes objects. */
export interface UpdateManyOptions<Fields extends string, Match extends string> {
    /** The fields the call writes on each matched object. */
    readonly fields: readonly Fields[];
    /** The fields the call matches objects by. */
    readonly match: readonly Match[];
    /** The input fields each release computes from an earlier call's input, by the release introducing them. */
    readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
}

/** How a deletion method deletes. */
export interface RemoveOptions<Prepared extends schema.Schema> {
    /** Whether only the system deletes, with no permission. */
    readonly isSystem?: true;
    /** The external work the deletion prepares before its transaction. */
    readonly prepared?: Prepared;
}

/** Create an object from the fields the caller writes. */
export function create<
    const Permission extends string | null,
    const Fields extends string = string,
    Input extends schema.JsonObject = never,
    Prepared extends schema.Schema = never,
    const System extends boolean = false,
    Definition extends Table = Table,
>(
    permission: Permission,
    options: CreateOptions<Fields, Input, Prepared, System, Definition> = {},
): Method<{
    kind: "create";
    permission: Permission;
    input: NoInfer<Input>;
    prepared: NoInfer<Prepared>;
    mutates: true;
    fields: NoInfer<Fields>;
    table: Definition;
    system: NoInfer<System>;
}> {
    // resolve the creation from a creator relation or a declared creation
    const { creator: _creator, creation: _creation, ...declared } = options;
    const creation = resolveCreation(options);

    return defineMethod<{
        kind: "create";
        permission: Permission;
        input: Input;
        prepared: Prepared;
        mutates: true;
        fields: Fields;
        table: Definition;
        system: System;
    }>({
        kind: "create",
        permission,
        mutates: true,
        ...declared,
        target: false,
        result: "object",
        procedure: (_name, shapes) => createProcedure(shapes, options),
        handler: createObject,
        authorize: (call: Call<ObjectTable>) => authorizeCreate(call, creation),
        inverse: (step) => {
            // delete the created object
            const call = Step.call(step, "delete", Step.target(step));

            return call === undefined ? undefined : [call];
        },
        async execute(call: Call<ObjectTable>) {
            // predict under the client's identifier
            if (call.isPredicted) {
                if (call.id === undefined) {
                    throw new TypeError(
                        `predicted ${call.object.name} creations need an identifier`,
                    );
                }

                return this.handler(call);
            }

            // insert under an unused identifier and record its access
            return insertAuthorized(call, creation, (inserted) => this.handler(inserted));
        },
    });
}

/** Update an object's written fields. */
export function update<
    const Permission extends string | null,
    const Fields extends string = string,
    Input extends schema.JsonObject = never,
    Prepared extends schema.Schema = never,
    Definition extends Table = Table,
>(
    this: MethodBuilder<Definition>,
    permission: Permission,
    options: UpdateOptions<Fields, Input, Prepared> = {},
): Method<{
    kind: "update";
    permission: Permission;
    input: NoInfer<Input>;
    prepared: NoInfer<Prepared>;
    mutates: true;
    fields: NoInfer<Fields>;
    table: Definition;
}> {
    return defineMethod<{
        kind: "update";
        permission: Permission;
        input: Input;
        prepared: Prepared;
        mutates: true;
        fields: Fields;
        table: Definition;
    }>({
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
        handler: (call: Call<ObjectTable>) =>
            call.update(call.object.table[TABLE].decode(call.input)),
        inverse: restoreUpdated,
        async execute(call) {
            // refuse managed objects and unwritable fields
            requireUnmanaged(call);
            if (!call.isPredicted) {
                await call.requireAuthorization().requireWritable(call, call.requireId());
            }

            return this.handler(call);
        },
    });
}

/** Update every matching object the caller may change, returning the count. */
export function updateMany<
    const Permission extends string,
    const Fields extends string = string,
    const Match extends string = string,
    Definition extends Table = Table,
>(
    this: MethodBuilder<Definition>,
    permission: Permission,
    options: UpdateManyOptions<Fields, Match>,
): Method<{
    kind: "updateMany";
    permission: Permission;
    output: typeof BatchPayload;
    mutates: true;
    fields: NoInfer<Fields>;
    table: Definition;
}> {
    return defineMethod<{
        kind: "updateMany";
        permission: Permission;
        output: typeof BatchPayload;
        mutates: true;
        fields: Fields;
        table: Definition;
    }>({
        kind: "updateMany",
        permission,
        output: BatchPayload,
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
            output: BatchPayload,
        }),
        validate: (object) => requireUnrecorded(object, options.fields),
        handler: (call: Call<ObjectTable>) => updateMatching(call, options.fields),
    });
}

/** Delete an object. */
export function remove<
    const Permission extends string | null,
    Prepared extends schema.Schema = never,
    Definition extends Table = Table,
>(
    this: MethodBuilder<Definition>,
    permission: Permission,
    options: RemoveOptions<Prepared> = {},
): Method<{
    kind: "delete";
    permission: Permission;
    output: typeof Empty;
    prepared: NoInfer<Prepared>;
    mutates: true;
    table: Definition;
}> {
    return defineMethod<{
        kind: "delete";
        permission: Permission;
        output: typeof Empty;
        prepared: Prepared;
        mutates: true;
        table: Definition;
    }>({
        kind: "delete",
        permission,
        output: Empty,
        mutates: true,
        ...options,
        target: true,
        result: "value",
        procedure: (_name, shapes) => ({
            route: { method: "DELETE", path: "/{id}" },
            input: shapes.target.extend({ ...shapes.replay, ...RevisionShape }),
            output: Empty,
        }),
        handler: deleteObject,
        inverse: (step) => {
            // restore from the trash
            const call =
                step.object.recoverable === undefined
                    ? undefined
                    : Step.call(step, "restore", Step.target(step));

            return call === undefined ? undefined : [call];
        },
        async execute(call) {
            requireUnmanaged(call);

            return this.handler(call);
        },
    });
}

/** A custom method's permission, input, output and inverse. */
export interface CustomDefinition<
    Permission extends string | null,
    Input extends schema.JsonObject,
    Output extends schema.Schema,
    Prepared extends schema.Schema,
    System extends boolean = false,
> {
    /** The permission the caller needs on the target, null for none. */
    readonly permission: Permission;
    /** Whether only the system calls the method. */
    readonly isSystem?: System;
    /** The caller-supplied fields. */
    readonly input?: Input;
    /** The result, the updated object when absent. */
    readonly output?: Output;
    /** The external work the method prepares before its transaction. */
    readonly prepared?: Prepared;
    /** Whether each reading call is audited. */
    readonly audited?: true;
    /** The fields of the result an audited read's event names. */
    readonly audit?: { readonly details: schema.Object<Record<string, schema.Schema>> };
    /** The inverse method name, or a function deriving inverse calls. */
    readonly inverse?: string | ((step: Step) => readonly sync.Call[] | undefined);
    /** The input fields each release computes from an earlier call's input, by the release introducing them. */
    readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
}

/** Declare a method reading one object, as tRPC's `query`. */
export function query<
    const Permission extends string | null,
    Input extends schema.JsonObject = never,
    Output extends schema.Schema = never,
    Prepared extends schema.Schema = never,
    const System extends boolean = false,
    Definition extends Table = Table,
>(
    this: MethodBuilder<Definition>,
    definition: CustomDefinition<Permission, Input, Output, Prepared, System>,
): Method<{
    kind: "custom";
    permission: Permission;
    input: Input;
    output: Output;
    prepared: Prepared;
    mutates: false;
    table: Definition;
    system: System;
}> {
    return defineMethod<{
        kind: "custom";
        permission: Permission;
        input: Input;
        output: Output;
        prepared: Prepared;
        mutates: false;
        table: Definition;
        system: System;
    }>(custom(definition, false));
}

/** Declare a method changing one object, as tRPC's `mutation`. */
export function mutation<
    const Permission extends string | null,
    Input extends schema.JsonObject = never,
    Output extends schema.Schema = never,
    Prepared extends schema.Schema = never,
    const System extends boolean = false,
    Definition extends Table = Table,
>(
    this: MethodBuilder<Definition>,
    definition: CustomDefinition<Permission, Input, Output, Prepared, System>,
): Method<{
    kind: "custom";
    permission: Permission;
    input: Input;
    output: Output;
    prepared: Prepared;
    mutates: true;
    table: Definition;
    system: System;
}> {
    return defineMethod<{
        kind: "custom";
        permission: Permission;
        input: Input;
        output: Output;
        prepared: Prepared;
        mutates: true;
        table: Definition;
        system: System;
    }>(custom(definition, true));
}

/** Describe a custom method on one object, reading or changing it. */
function custom<
    const Permission extends string | null,
    Input extends schema.JsonObject,
    Output extends schema.Schema,
    Prepared extends schema.Schema,
    const Mutates extends boolean,
    const System extends boolean,
>(
    definition: CustomDefinition<Permission, Input, Output, Prepared, System>,
    mutates: Mutates,
): MethodDefinition<{
    kind: "custom";
    permission: Permission;
    input: Input;
    output: Output;
    prepared: Prepared;
    mutates: Mutates;
    system: System;
}> {
    const { inverse, ...declared } = definition;

    return {
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
        handler: missingHandler,
    };
}

/** Read a method's own input fields. */
function fields(input: schema.JsonObject | undefined): schema.JsonObject["shape"] {
    return input?.shape ?? {};
}

/** Derive a creation's procedure: the scope, the chosen identifier, the parent and the written fields. */
function createProcedure(
    shapes: ObjectSchema,
    options: Pick<
        CreateOptions<string, schema.JsonObject, schema.Schema, boolean>,
        "fields" | "isSystem" | "input"
    >,
): MethodProcedure {
    return {
        route: { method: "POST", path: "" },
        input: shapes.scope.extend({
            ...shapes.created,
            ...shapes.parent,
            ...shapes.replay,
            ...shapes.written(options.fields, false),
            ...(options.isSystem === true ? shapes.callers : {}),
            ...fields(options.input),
        }),
        output: shapes.row,
    };
}

/** Resolve a creation from a creator relation or a declared creation, refusing both. */
function resolveCreation(
    options: Pick<
        CreateOptions<string, schema.JsonObject, schema.Schema, boolean>,
        "creator" | "creation"
    >,
): CreateOptions<string, schema.JsonObject, schema.Schema, boolean>["creation"] {
    // refuse a creator relation beside a creation
    const { creator, creation } = options;
    if (creator !== undefined && creation !== undefined) {
        throw new TypeError("a creation names a creator relation or a creation, not both");
    }

    return creator === undefined ? creation : relateCreator(creator);
}

/** Require the receiving parent and the create permission of a caller. */
async function authorizeCreate(
    call: Call<ObjectTable>,
    creation: ((call: Call, object: ObjectReference) => Creation | Promise<Creation>) | undefined,
): Promise<void> {
    // skip a creation needing no permission
    if (call.method.permission === null) {
        return;
    }

    // require the receiving parent and the creation
    const reader = await call.requireAuthorization().reader(call.scope);
    const parent = call.object.parent && call.parent();
    if (parent) {
        await call.requireReceiving(parent, reader);
    }
    await requireCreatable(call, reader, creationOf(call, creation));
}

/** Insert a created object under an unused identifier, record its access and require guarded writes. */
async function insertAuthorized(
    call: Call<ObjectTable>,
    creation: ((call: Call, object: ObjectReference) => Creation | Promise<Creation>) | undefined,
    insert: (created: Call<ObjectTable>) => Promise<unknown>,
): Promise<unknown> {
    // insert under an unused identifier
    const authorization = call.requireAuthorization();
    const { id, created, row } = await insertCreated(call, insert);

    // record access and require guarded writes
    const object = call.object.reference(call.scope, id);
    const isScope = call.object.policy.definition.scope === true;
    const creating = creationOf(call, creation);
    if (isScope || creating !== undefined) {
        await authorization.create(object, (await creating?.(created, object)) ?? {});
    }
    await authorization.requireWritable(created, id);

    return row;
}

/** Restore the updated fields a step wrote that stayed unchanged since. */
function restoreUpdated(step: Step): readonly sync.Call[] | undefined {
    // restore nothing for a deleted object
    const current = step.current;
    if (current === undefined) {
        return undefined;
    }

    // restore the fields unchanged since
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
        Step.record(step, step.name, {
            ...Step.target(step),
            ...restored,
        }),
    ];
}

/** Refuse tracked, declarable and versioned objects and guarded fields, whose changes many updates at once skip. */
function requireUnrecorded(object: ObjectType, written: readonly string[]): void {
    // refuse objects keeping a record of each change
    if (
        object.tracked !== undefined ||
        object.declarationSchema !== undefined ||
        object.versioned
    ) {
        throw new TypeError(
            `object ${object.name} keeps a record of each change, which many updates at once skip`,
        );
    }

    // refuse guarded fields
    const guarded = written.find((name) => object.fields[name]?.access?.write !== undefined);
    if (guarded !== undefined) {
        throw new TypeError(
            `object ${object.name} guards field ${guarded}, which many updates at once skip`,
        );
    }
}

/** Update every matching object the caller may change, returning the count. */
async function updateMatching(
    call: Call<ObjectTable>,
    written: readonly string[],
): Promise<{ count: number }> {
    // decode values and matches
    const table = call.object.table;
    const definition = table[TABLE];
    const input: JsonObject = call.input;
    const values = definition.decode(
        Object.fromEntries(
            written.flatMap((name) => {
                const value = input[name];

                return value === undefined ? [] : [[name, value]];
            }),
        ),
    );
    const matched = definition.decode(MATCH.parse(input["where"]));
    const hasGeneration = Object.hasOwn(definition.columns, "generation");

    // match by the given values, restricted to permitted objects
    const matching = and(
        call.object.inScope(call.scope),
        ...Object.entries(matched).map(([name, value]) =>
            value === null ? isNull(definition.column(name)) : eq(definition.column(name), value),
        ),
    );
    const permitted = call.isPredicted ? undefined : await permittedMatches(call, matching);

    // update the permitted matches
    const columns: Table = table;
    const changed = await call.database
        .update(columns)
        .set({
            ...values,
            revision: sql`${table.revision} + 1`,
            ...(hasGeneration ? { generation: sql`${definition.column("generation")} + 1` } : {}),
            updatedAt: call.now,
            updatedBy: call.caller === undefined ? null : Subject.key(call.caller),
        })
        .where(and(matching, permitted))
        .returning({ id: table.id });

    return { count: changed.length };
}

/** Restrict a served call's matches to the objects the caller may change. */
async function permittedMatches(
    call: Call<ObjectTable>,
    matching: SQL | undefined,
): Promise<SQL | undefined> {
    // decide in SQL where the permission compiles
    const served = call.requireAuthorization();
    const required = call.permission();
    const listable = served.listable(call.object, required);
    if (listable !== "memory") {
        return listable;
    }

    // keep the matched rows decided in memory
    const table = call.object.table;
    const rows = await call.database.select().from(table).where(matching);
    const kept = await served.keep(rows, required);

    return inArray(
        table.id,
        kept.map((row) => row.id),
    );
}

/** Insert a created object. */
async function createObject(call: Call<ObjectTable>): Promise<Select<Table>> {
    // insert the created columns
    const columns: Table = call.object.table;
    const [row] = await call.database
        .insert(columns)
        .values(await createdValues(call))
        .returning();
    if (row === undefined) {
        throw new TypeError(`the insert of ${call.object.name} returned no row`);
    }

    return row;
}

/** Build the columns a created object is inserted with. */
async function createdValues(call: Call<ObjectTable>): Promise<InsertValue<Table>> {
    // fill caller fields with a calling principal of the declared kinds
    const { object, caller } = call;
    const isSystem = call.method.isSystem === true;
    const callers: Record<string, string> = {};
    for (const [name, declared] of Object.entries(object.fields)) {
        const kinds = declared.principals;
        if (!declared.isCallerFilled || isSystem) {
            continue;
        } else if (
            caller === undefined ||
            !(kinds === undefined ? isPrincipal(caller) : kinds.some((kind) => kind.is(caller)))
        ) {
            const named = kinds?.map((kind) => kind.name).join(" or ") ?? "principal";
            throw new ServiceError("FORBIDDEN", {
                message: `${object.name} is created by a ${named}`,
            });
        }
        callers[name] = declared.type === "subject" ? Subject.key(caller) : caller.id;
    }

    // number versions within their parent
    const version = object.versioned
        ? await versioned.next(object, call.requireParent().id, call.database)
        : undefined;

    // require the writing client
    if (object.storage === "ephemeral" && call.client === undefined) {
        throw new TypeError(`ephemeral ${object.name} objects are created by a client`);
    }

    // assemble the columns
    return {
        ...object.table[TABLE].decode(call.input),
        ...(object.parent === undefined ? {} : call.parentColumns()),
        id: call.id,
        scope: call.scope,
        ...(version === undefined ? {} : { number: version }),
        ...(object.storage === "ephemeral" ? { client: call.client } : {}),
        ...callers,
        createdAt: call.now,
        createdBy: caller === undefined ? null : Subject.key(caller),
        updatedAt: call.now,
        updatedBy: caller === undefined ? null : Subject.key(caller),
    };
}

/** Insert a created object under an unused identifier within a savepoint. */
async function insertCreated(
    call: Call<ObjectTable>,
    insert: (created: Call<ObjectTable>) => Promise<unknown>,
): Promise<{ readonly id: string; readonly created: Call<ObjectTable>; readonly row: unknown }> {
    // insert a prediction directly
    if (call.isPredicted) {
        const id = await createdId(call);
        const created = call.with({ id });

        return { id, created, row: await insert(created) };
    }

    // insert within a savepoint
    try {
        return await call.database.transaction(async (transaction) => {
            // insert under an unused identifier
            const inserting = call.with({ database: transaction });
            const id = await createdId(inserting);
            const row = await insert(inserting.with({ id }));

            return { id, created: call.with({ id }), row };
        });
    } catch (error) {
        // report conflicts only to admitted callers and the system
        const isConflict =
            (error instanceof ServiceError && error.code === "CONFLICT") ||
            (error instanceof DatabaseError &&
                (error.code === "DUPLICATE" || error.code === "BROKEN_REFERENCE"));
        if (
            isConflict &&
            call.method.permission !== null &&
            !call.requireAuthorization().isSystem
        ) {
            await requireCreatable(call);
        }
        throw error;
    }
}

/** Read a created object's column values as a decision reads them before the row is written, leaving out values only the database computes. */
function createdRow(table: Table, values: InsertValue<Table>): Row {
    const row: Record<string, ColumnValue> = {};
    for (const [name, column] of Object.entries(table[TABLE].columns)) {
        // take the written value, else the column's default
        const written: unknown = Object.hasOwn(values, name) ? values[name] : undefined;
        const value = written === undefined ? column.definition.default : written;

        // keep a value, and leave out a missing one or one the database computes
        if (value === null) {
            row[name] = null;
        } else if (value !== undefined && !isSQLWrapper(value)) {
            row[name] = ColumnValue.parse(column.definition.schema.parse(value));
        }
    }

    return row;
}

/** Require the create permission on the object as written. */
async function requireCreatable(
    call: Call<ObjectTable>,
    reader?: GrantReader,
    creation?: (call: Call, object: ObjectReference) => Creation | Promise<Creation>,
): Promise<void> {
    // build the row as it will be written, with column defaults
    const id = call.id ?? call.object.generateId();
    const row = createdRow(call.object.table, await createdValues(call.with({ id })));

    // check the permission on the row
    const authorization = call.requireAuthorization();
    const { authorizer, snapshot } = authorization;
    const grants = reader ?? (await authorization.reader(call.scope));
    const created = call.object.reference(call.scope, id);
    const initial = (await creation?.(call.with({ id }), created)) ?? {};
    grants.creating(created, authorizer.initialRelationships(created, initial, call.now));
    const permission = call.permission();
    const admits = async (access: Access) =>
        (await authorizer.checkRows(snapshot, permission, access, [row], grants)).permitted.has(0);
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
        (await authorizer.checkRows(snapshot, reading, access, [row])).permitted.has(0);

    // refuse a reader, and hide the object from others
    const denial = new ServiceError("FORBIDDEN", {
        message: `permission denied: ${permission.name}`,
    });
    throw isReadable ? denial : conceal(denial, `${call.object.name} not found`);
}

/** Delete an object, or request deletion when a trash or controller finishes it. */
async function deleteObject(call: Call<ObjectTable>): Promise<Record<string, never>> {
    // refuse deleting an object twice
    const { object } = call;
    const target: Row = call.requireTarget();
    if (target["deletionRequestedAt"] !== undefined && target["deletionRequestedAt"] !== null) {
        throw new ServiceError("CONFLICT", { message: `${object.name} is already deleted` });
    }

    // request deletion when a trash or controller finishes it
    const deleting = deletingCall(call);
    if (deleting !== undefined) {
        await deleting.update({
            deletionRequestedAt: call.now,
            deletedBy: call.caller === undefined ? null : Subject.key(call.caller),
        });

        return {};
    }

    // delete at the loaded revision
    await call.remove();

    return {};
}

/** Read a call by the deletion columns its object's table keeps, absent for an object deleting at once. */
function deletingCall(call: Call<ObjectTable>): Call<RecoverableTable> | undefined;
/**
 * Read a call by its table's deletion columns, for objects a trash or a controller finishes deleting.
 *
 * @construct defineObject derives the deletion columns into the table of every object whose definition sets `recoverable` or `controlled`.
 */
function deletingCall(call: Call<ObjectTable>): Call | undefined {
    return call.object.recoverable === undefined && !call.object.isControlled ? undefined : call;
}

/** Accept a caller's unused identifier for a created object, or generate one. */
async function createdId(call: Call<ObjectTable>): Promise<string> {
    // generate a missing identifier
    const { object, id: chosen } = call;
    if (chosen === undefined) {
        return object.generateId();
    }

    // find a row with the identifier, or a relationship a deleted object left under it
    const table = object.table;
    const { packageId, name } = object.policy.definition;
    const rows = sql`SELECT 1 FROM ${table} WHERE ${eq(table[TABLE].column("id"), chosen)}`;
    const relationships = sql`SELECT 1 FROM ${accessRelationship} WHERE ${and(
        eq(accessRelationship.packageId, packageId),
        eq(accessRelationship.type, name),
        eq(accessRelationship.objectId, chosen),
    )}`;
    const isTaken =
        object.storage === "durable"
            ? await call.database.exists(sql`${rows} UNION ALL ${relationships}`)
            : (await call.database.exists(rows)) ||
              (await call.requireAuthorization().database.exists(relationships));

    // refuse a taken identifier
    if (isTaken) {
        throw new ServiceError("CONFLICT", { message: `${object.name} identifier is taken` });
    }

    return chosen;
}

/** Read a page of a query's rows or its aggregate groups. */
export async function listObjects(call: Call) {
    // bind the cursor to the query
    const { cursor, limit, at: _at, branch: _branch, ...options } = LIST_INPUT.parse(call.input);
    const listing = new Page(
        { ...(cursor === undefined ? {} : { cursor }), ...(limit === undefined ? {} : { limit }) },
        [call.object.name, call.scope, canonicalize(options)],
        schema.record(schema.string(), schema.json()),
    );

    // compile the query with one extra row
    const compiled = {
        ...call.object.query(
            options.aggregate === undefined ? { ...options, limit: listing.limit + 1 } : options,
            call.objects,
        ),
        scopes: call.object.scopesOf(call.chain),
    };
    const dataflow = listingDataflow(call, compiled);
    const node = aligned(dataflow.roots, 0);

    // read groups at the latest position, or rows
    const position = call.snapshot?.position;
    const after = listing.after === undefined ? undefined : node.table[TABLE].decode(listing.after);
    await dataflow.load(await View.of(call.database, call.snapshot), after);
    if (node.aggregate !== undefined && position !== undefined) {
        throw new ServiceError("BAD_REQUEST", {
            message: "aggregates read the latest position, whose access decides them",
        });
    } else if (node.aggregate !== undefined) {
        return { items: [], cursor: null, groups: await dataflow.results("list") };
    }

    // keep the items the caller could read and split includes and computed values
    const kept = await keepReadable(call, await dataflow.read("list"));
    const listed = listing.result(kept, (item) =>
        node.table[TABLE].encode(node.positionOf(item.row)),
    );

    return { ...listed, ...splitListed(listed.items, node, compiled) };
}

/** Build the dataflow listing a call's query, materialized for the call's audience. */
function listingDataflow(call: Call, compiled: sync.Query): Dataflow {
    return new Dataflow(
        { list: compiled },
        {
            audience: new Listing(call),
            database: call.database,
            changesThrough: changesThroughLog(call.database),
            isMaterialized: true,
        },
    );
}

/** Keep the items whose rows the caller could read at the snapshot position. */
async function keepReadable(call: Call, read: readonly sync.Item[]): Promise<readonly sync.Item[]> {
    // keep every item at the latest position
    const { snapshot } = call;
    if (snapshot?.position === undefined) {
        return read;
    }

    // keep the items whose rows access permits at the snapshot position
    const permitted = new Set(
        await call.requireAuthorization().keepAt(
            read.map((item) => item.row),
            call.permission(),
            snapshot,
            call.scope,
        ),
    );

    return read.filter((item) => permitted.has(item.row));
}

/** Split listed items into their rows, their included rows and their computed values. */
function splitListed(listed: readonly sync.Item[], node: sync.Node, compiled: sync.Query) {
    // list the included relations and the computed values
    const names = node.children
        .filter((child) => child.kind === "include")
        .map((child) => child.name.slice(node.name.length + 1));
    const extras = Object.keys(node.extras);
    const values = Object.fromEntries(
        listed.map(({ row }) => [
            schema.string().parse(row["id"]),
            Object.fromEntries(extras.map((name) => [name, node.scalar(name, row[name])])),
        ]),
    );

    // index the included rows by relation and item
    const nestedRows = listed.map((item) => nest(item, compiled));
    const included = Object.fromEntries(
        names.map((name) => [
            name,
            Object.fromEntries(
                nestedRows.map((row) => [
                    schema.string().parse(row["id"]),
                    schema.json().parse(row[name]),
                ]),
            ),
        ]),
    );

    // strip the computed values from the rows
    const items = listed.map(({ row }) =>
        Object.fromEntries(Object.entries(row).filter(([name]) => !extras.includes(name))),
    );

    return {
        items,
        ...(names.length === 0 ? {} : { included }),
        ...(extras.length === 0 ? {} : { extras: values }),
    };
}

/** Refuse standard changes to managed objects. */
function requireUnmanaged(call: Call): void {
    if (Manager.isManaging(call.target)) {
        throw new ServiceError("MANAGED", {
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
        schema.Object<ReturnType<typeof page<RowSchema<Object>>>["shape"] & typeof ListShape>
    >;
    create: Procedure<
        schema.Object<
            ScopeShape<Object> &
                CreationShape<Object> &
                ParentShape<Object> &
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

/** The written values a call matches objects by, in their JSON form. */
type MatchShape<Object extends ObjectType> = WrittenShape<Object, Method, true>;

/** The identifier a caller may choose for a created object. */
type CreationShape<Object extends ObjectType> = {
    id: schema.ExactOptional<
        schema.Schema<JsonOf<Select<Object["table"]>["id" & keyof Select<Object["table"]>]>>
    >;
};

/** Read how a call creates its object's first relationships: as declared, else relating the creator as owner of a scope shared through the roles. */
function creationOf(
    call: Call,
    declared: ((call: Call, object: ObjectReference) => Creation | Promise<Creation>) | undefined,
): ((call: Call, object: ObjectReference) => Creation | Promise<Creation>) | undefined {
    const isOwnedScope =
        call.object.roles !== undefined && call.object.policy.definition.scope === true;

    return declared ?? (isOwnedScope ? relateCreator("owner") : undefined);
}

/** Relate the calling principal to a new object, and a new scope's owner role to the relation's holders. */
function relateCreator(relation: string): (call: Call, object: ObjectReference) => Creation {
    return (call, object) => {
        // require a creator
        const caller = call.requireCaller();
        const isScope = call.object.policy.definition.scope === true;

        return {
            relationships: [{ relation, subject: caller }],
            ...(isScope ? { owner: { ...object, relation } } : {}),
        };
    };
}

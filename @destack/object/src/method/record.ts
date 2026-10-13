import { Orderable } from "../trait/orderable.ts";
import {
    AccessError,
    accessRelationship,
    isPrincipal,
    type Access,
    type Creation,
    type TupleReader,
} from "@destack/access";
import {
    TABLE,
    type InsertValue,
    type Select,
    type Table,
    type Expression,
    DatabaseError,
    Row,
    Condition,
    Scalar,
} from "@destack/db";
import { aligned, canonicalize, schema, type JsonObject, type Version } from "@destack/schema";
import { changesThroughLog, Dataflow, View, type ObjectReference, Subject } from "@destack/sync";
import { conceal, isCallError, CallError } from "@destack/runtime/error";
import { Page } from "./page.ts";
import { Call } from "./call.ts";
import { Step } from "./step.ts";
import type * as sync from "@destack/sync";
import { defineMethod, type Method, type MethodBuilder, type MethodDefinition } from "./method.ts";
import {
    Empty,
    PageShape,
    type MethodProcedure,
    type ObjectSchema,
    RevisionShape,
    type CustomSchema,
    type FieldShape,
    type ColumnShape,
    type WrittenShape,
    type IdSchema,
    type ParentShape,
    type Procedure,
    type ReplayShape,
    type RevisionField,
    type RowSchema,
    type ScopeShape,
    type TargetShape,
    type VariantSchema,
    ListPage,
} from "./procedure.ts";
import { InputSchema } from "./input.ts";
import { kebabCase } from "../object/name.ts";
import { EventListing } from "../query/event.ts";
import { Listing } from "../query/listing.ts";
import { nest } from "../query/item.ts";
import { ListShape, QueryShape, ViewShape } from "../replica/replica.ts";
import type { ObjectType, WrittenField } from "../object/object.ts";
import type { ObjectTable, TraitTable } from "../object/table.ts";

import { nextVersion } from "./versioned.ts";
import { isManaging } from "../trait/declarable.ts";

import type { RecoverableDefinition } from "../trait/recoverable.ts";

/** The most items one bulk creation takes, as many as one list page reads. */
export const BATCH_LIMIT = 1000;

/** The count of objects a bulk update or deletion changed. */
export const BatchPayload = schema.object({ count: schema.number().int().nonnegative() });

/** The identifiers of the objects a bulk creation created, in item order. */
export const BatchIds = schema.object({ ids: schema.array(schema.string()) });

/** The items of a bulk creation, each one creation's fields. */
export const BatchItems = schema.array(schema.record(schema.string(), schema.json()));

/** The query, page and view of a list's input, beside its scope fields. */
const LIST_INPUT = schema.looseObject({ ...QueryShape, ...PageShape, ...ViewShape });
/** The query, page and view of a list's input. */
export type ListInput = schema.Infer<typeof LIST_INPUT>;

/** The field values a bulk update matches objects by, by field name. */
const MATCH = schema.record(schema.string(), Scalar);

/** Read one object. */
export function get<const Permission extends string, Definition extends Table = Table>(
    this: MethodBuilder<Definition>,
    permission: Permission,
): Method<{ kind: "get"; permission: Permission; mutates: false; table: Definition }> {
    return defineMethod<{ kind: "get"; permission: Permission; mutates: false; table: Definition }>(
        {
            kind: "get",
            source: { builder: "get", arguments: [permission] },
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
        source: { builder: "list", arguments: [permission] },
        permission,
        mutates: false,
        target: false,
        result: "page",
        procedure: (_name, shapes) => ({
            route: { method: "POST", path: "/query" },
            input: shapes.scope.extend({ ...QueryShape, ...PageShape, ...ViewShape }),
            output: ListPage.schema(shapes.row).extend(ListShape),
        }),
        handler: listObjects,
    });
}

/** The column sets of a sum type, by discriminating field and then value. */
export type Variants<Fields extends string = string> = Readonly<
    Record<string, Readonly<Record<string, readonly Fields[]>>>
>;

/** How a creation method creates: its written fields, input, creator and prediction. */
export interface CreateOptions<
    Fields extends string,
    Input extends schema.JsonObject,
    Prepared extends schema.Schema,
    Internal extends boolean,
    Sum extends Variants = Variants,
> {
    /** Whether the creation stays off public routes, setting the caller fields itself. */
    readonly isInternal?: Internal;
    /** The fields the caller writes, every written field when absent. */
    readonly fields?: readonly Fields[];
    /** The column sets of a sum-typed row, of which the caller writes one beside the fields. */
    readonly variants?: Sum;
    /** The caller-supplied values beyond fields. */
    readonly input?: Input;
    /** The external work the creation prepares before its transaction. */
    readonly prepared?: Prepared;
    /** The relation the calling principal takes to the new object. */
    readonly creator?: string;
    /** Whether clients predict the creation. */
    readonly isPredicted?: false;
    /** The input fields each release computes from earlier input, by introducing release. */
    readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
}

/** How a bulk creation creates: its written fields and creator. */
export interface CreateManyOptions<Fields extends string, Internal extends boolean> {
    /** Whether the creation stays off public routes, setting the caller fields itself. */
    readonly isInternal?: Internal;
    /** The fields the caller writes on each item. */
    readonly fields: readonly Fields[];
    /** The relation the calling principal takes to each new object. */
    readonly creator?: string;
}

/** How an update method updates: its written fields, input and prediction. */
export interface UpdateOptions<
    Fields extends string,
    Input extends schema.JsonObject,
    Prepared extends schema.Schema,
> {
    /** Whether the update stays out of the routes people call. */
    readonly isInternal?: true;
    /** The fields the caller writes, every written field when absent. */
    readonly fields?: readonly Fields[];
    /** The caller-supplied values beyond fields. */
    readonly input?: Input;
    /** The external work the update prepares before its transaction. */
    readonly prepared?: Prepared;
    /** Whether clients predict the update. */
    readonly isPredicted?: false;
    /** The input fields each release computes from earlier input, by introducing release. */
    readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
}

/** How a bulk update matches and writes objects. */
export interface UpdateManyOptions<Fields extends string, Match extends string> {
    /** The fields the call writes on each matched object. */
    readonly fields: readonly Fields[];
    /** The fields the call matches objects by. */
    readonly match: readonly Match[];
    /** The input fields each release computes from earlier input, by introducing release. */
    readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
}

/** How a bulk deletion matches objects. */
export interface DeleteManyOptions<Match extends string> {
    /** The fields the call matches objects by. */
    readonly match: readonly Match[];
}

/** How a deletion method deletes. */
export interface RemoveOptions<Prepared extends schema.Schema> {
    /** Whether the deletion stays out of the routes people call. */
    readonly isInternal?: true;
    /** The external work the deletion prepares before its transaction. */
    readonly prepared?: Prepared;
}

/** Create an object from the fields the caller writes. */
export function create<
    const Permission extends string,
    const Fields extends string = string,
    Input extends schema.JsonObject = never,
    Prepared extends schema.Schema = never,
    const Internal extends boolean = false,
    Definition extends Table = Table,
    const Sum extends Variants = never,
>(
    this: MethodBuilder<Definition>,
    permission: Permission,
    options: CreateOptions<Fields, Input, Prepared, Internal, Sum> = {},
): Method<{
    kind: "create";
    permission: Permission;
    input: NoInfer<Input>;
    prepared: NoInfer<Prepared>;
    mutates: true;
    fields: NoInfer<Fields>;
    variants: NoInfer<Sum>;
    table: Definition;
    internal: NoInfer<Internal>;
}> {
    // relate the creator a creation names
    const { creator, ...declared } = options;

    return defineMethod<{
        kind: "create";
        permission: Permission;
        input: Input;
        prepared: Prepared;
        mutates: true;
        fields: Fields;
        variants: Sum;
        table: Definition;
        internal: Internal;
    }>({
        kind: "create",
        source: { builder: "create", arguments: [permission, options] },
        permission,
        mutates: true,
        ...declared,
        target: false,
        result: "object",
        procedure: (name, shapes) => createProcedure(name, shapes, options),
        ...(creator === undefined ? {} : { creation: relateCreator(creator) }),
        handler: (call: Call<ObjectTable>) => createObject(call),
        authorize: (call: Call<ObjectTable>) => authorizeCreate(call),
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
            const { row } = await insertAuthorized(call, (inserted) => this.handler(inserted));

            return row;
        },
    });
}

/** Create one object per item from the fields the caller writes. */
export function createMany<
    const Permission extends string,
    const Fields extends string = string,
    const Internal extends boolean = false,
    Definition extends Table = Table,
>(
    this: MethodBuilder<Definition>,
    permission: Permission,
    options: CreateManyOptions<Fields, Internal>,
): Method<{
    kind: "createMany";
    permission: Permission;
    output: typeof BatchIds;
    mutates: true;
    fields: NoInfer<Fields>;
    table: Definition;
    internal: NoInfer<Internal>;
}> {
    // relate the creator a creation names
    const { creator, ...declared } = options;

    return defineMethod<{
        kind: "createMany";
        permission: Permission;
        output: typeof BatchIds;
        mutates: true;
        fields: Fields;
        table: Definition;
        internal: Internal;
    }>({
        kind: "createMany",
        source: { builder: "createMany", arguments: [permission, options] },
        permission,
        output: BatchIds,
        mutates: true,
        ...declared,
        target: false,
        result: "value",
        procedure: (name, shapes) => ({
            route: { method: "POST", path: `/${kebabCase(name)}` },
            input: shapes.scope.extend({
                ...shapes.replay,
                items: schema
                    .array(
                        schema.object({
                            ...shapes.created,
                            ...shapes.parent,
                            ...shapes.written(options.fields, false),
                            ...(options.isInternal === true ? shapes.callers : {}),
                        }),
                    )
                    .max(BATCH_LIMIT),
            }),
            output: BatchIds,
        }),
        validate: requireUnrecorded,
        ...(creator === undefined ? {} : { creation: relateCreator(creator) }),
        handler: (call: Call<ObjectTable>) => createItems(call),
        authorize: (call: Call<ObjectTable>) => authorizeItems(call),
        inverse: deleteCreated,
    });
}

/** Update an object's written fields. */
export function update<
    const Permission extends string,
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
        source: { builder: "update", arguments: [permission, options] },
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
    match: NoInfer<Match>;
    fields: NoInfer<Fields>;
    table: Definition;
}> {
    return defineMethod<{
        kind: "updateMany";
        permission: Permission;
        output: typeof BatchPayload;
        mutates: true;
        match: Match;
        fields: Fields;
        table: Definition;
    }>({
        kind: "updateMany",
        source: { builder: "updateMany", arguments: [permission, options] },
        permission,
        output: BatchPayload,
        mutates: true,
        ...(options.convert === undefined ? {} : { convert: options.convert }),
        target: false,
        result: "value",
        procedure: (name, shapes) => ({
            route: { method: "PATCH", path: name === "updateMany" ? "" : `/${kebabCase(name)}` },
            input: shapes.scope.extend({
                ...shapes.replay,
                where: schema.object(shapes.written(options.match, true)),
                ...shapes.written(options.fields, true),
            }),
            output: BatchPayload,
        }),
        validate: (object) => {
            requireUnrecorded(object);
            requireUnguarded(object, options.fields);
        },
        handler: (call: Call<ObjectTable>) => updateMatching(call, options.fields),
    });
}

/** Delete an object. */
export function remove<
    const Permission extends string,
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
        source: { builder: "delete", arguments: [permission, options] },
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
                step.object.lifecycle.recoverable === undefined
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

/** Delete every matching object the caller may delete, returning the count. */
export function deleteMany<
    const Permission extends string,
    const Match extends string = string,
    Definition extends Table = Table,
>(
    this: MethodBuilder<Definition>,
    permission: Permission,
    options: DeleteManyOptions<Match>,
): Method<{
    kind: "deleteMany";
    permission: Permission;
    output: typeof BatchPayload;
    mutates: true;
    match: NoInfer<Match>;
    table: Definition;
}> {
    return defineMethod<{
        kind: "deleteMany";
        permission: Permission;
        output: typeof BatchPayload;
        mutates: true;
        match: Match;
        table: Definition;
    }>({
        kind: "deleteMany",
        source: { builder: "deleteMany", arguments: [permission, options] },
        permission,
        output: BatchPayload,
        mutates: true,
        target: false,
        result: "value",
        procedure: (name, shapes) => ({
            route: { method: "DELETE", path: name === "deleteMany" ? "" : `/${kebabCase(name)}` },
            input: shapes.scope.extend({
                ...shapes.replay,
                where: schema.object(shapes.written(options.match, true)),
            }),
            output: BatchPayload,
        }),
        validate: requireUnrecorded,
        handler: (call: Call<ObjectTable>) => deleteMatching(call),
    });
}

/** What a custom method acts on. */
export type CustomTarget = true | false | "none";

/** A custom method's permission, input, output and inverse. */
export interface CustomDefinition<
    Permission extends string,
    Input extends InputSchema,
    Output extends schema.Schema,
    Prepared extends schema.Schema,
    Internal extends boolean = false,
    Target extends CustomTarget = true,
> {
    /** The permission the caller needs on the target. */
    readonly permission: Permission;
    /** Whether the method stays off public routes, run only by the principals' own code. */
    readonly isInternal?: Internal;
    /** The caller-supplied fields. */
    readonly input?: Input;
    /** The result, the updated object when absent. */
    readonly output?: Output;
    /** The external work the method prepares before its transaction. */
    readonly prepared?: Prepared;
    /** Whether each reading call is audited. */
    readonly audited?: true;
    /** The fields of the result an audited read's event records. */
    readonly audit?: { readonly details: schema.Object<Record<string, schema.Schema>> };
    /** What the method acts on: by default its target, loaded under the caller's permission. */
    readonly target?: Target;
    /** The columns of every object a scoped method acts on, else the input's columns. */
    readonly selected?: (input: Readonly<Record<string, unknown>>) => Row;
    /** The inverse method name, or a function deriving inverse calls. */
    readonly inverse?: string | ((step: Step) => readonly sync.Call[] | undefined);
    /** The input fields each release computes from earlier input, by introducing release. */
    readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
}

/** Declare a method reading one object, as tRPC's `query`. */
export function query<
    const Permission extends string,
    Input extends InputSchema = never,
    Output extends schema.Schema = never,
    Prepared extends schema.Schema = never,
    const Internal extends boolean = false,
    const Target extends CustomTarget = true,
    Definition extends Table = Table,
>(
    this: MethodBuilder<Definition>,
    definition: CustomDefinition<Permission, Input, Output, Prepared, Internal, Target>,
): Method<{
    kind: "custom";
    permission: Permission;
    input: Input;
    output: Output;
    prepared: Prepared;
    mutates: false;
    table: Definition;
    internal: Internal;
    scoped: Target extends "none" ? true : false;
}> {
    return defineMethod<{
        kind: "custom";
        permission: Permission;
        input: Input;
        output: Output;
        prepared: Prepared;
        mutates: false;
        table: Definition;
        internal: Internal;
        scoped: Target extends "none" ? true : false;
    }>(custom(definition, false));
}

/** Declare a method changing one object, as tRPC's `mutation`. */
export function mutation<
    const Permission extends string,
    Input extends InputSchema = never,
    Output extends schema.Schema = never,
    Prepared extends schema.Schema = never,
    const Internal extends boolean = false,
    const Target extends CustomTarget = true,
    Definition extends Table = Table,
>(
    this: MethodBuilder<Definition>,
    definition: CustomDefinition<Permission, Input, Output, Prepared, Internal, Target>,
): Method<{
    kind: "custom";
    permission: Permission;
    input: Input;
    output: Output;
    prepared: Prepared;
    mutates: true;
    table: Definition;
    internal: Internal;
    scoped: Target extends "none" ? true : false;
}> {
    return defineMethod<{
        kind: "custom";
        permission: Permission;
        input: Input;
        output: Output;
        prepared: Prepared;
        mutates: true;
        table: Definition;
        internal: Internal;
        scoped: Target extends "none" ? true : false;
    }>(custom(definition, true));
}

/** Describe a custom method on one object, reading or changing it. */
function custom<
    const Permission extends string,
    Input extends InputSchema,
    Output extends schema.Schema,
    Prepared extends schema.Schema,
    const Mutates extends boolean,
    const Internal extends boolean,
    const Target extends CustomTarget,
>(
    definition: CustomDefinition<Permission, Input, Output, Prepared, Internal, Target>,
    mutates: Mutates,
): MethodDefinition<{
    kind: "custom";
    permission: Permission;
    input: Input;
    output: Output;
    prepared: Prepared;
    mutates: Mutates;
    internal: Internal;
    scoped: Target extends "none" ? true : false;
}> {
    // act in the scope for a method without a target
    const { inverse, target, selected, ...declared } = definition;
    const isScoped = target === "none";

    return {
        kind: "custom",
        source: { builder: mutates ? "mutation" : "query", arguments: [definition] },
        ...declared,
        ...(isScoped
            ? { authorize: (call: Call<ObjectTable>) => authorizeScoped(call, selected) }
            : {}),
        isScoped: isScopedTarget(target),
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
        target: target === undefined || target === true,
        result: definition.output === undefined ? "object" : "value",
        procedure: (name, shapes) => {
            // select the target, else the scope
            const selection = isScoped ? shapes.scope : shapes.target;

            return {
                route: {
                    method: "POST",
                    path: isScoped ? `/${kebabCase(name)}` : `/{id}/${kebabCase(name)}`,
                },
                input: InputSchema.map(definition.input ?? schema.object({}), (input) =>
                    selection.extend({ ...(mutates ? shapes.replay : {}), ...input.shape }),
                ),
                output: definition.output ?? shapes.row,
            };
        },
        handler: missingHandler,
    };
}

/** Read a method's own input fields. */
function fields(input: schema.JsonObject | undefined): schema.JsonObject["shape"] {
    return input?.shape ?? {};
}

/** Derive a creation's procedure at the collection, or by name below it for another creation. */
function createProcedure(
    name: string,
    shapes: ObjectSchema,
    options: Pick<
        CreateOptions<string, schema.JsonObject, schema.Schema, boolean>,
        "fields" | "isInternal" | "input" | "variants"
    >,
): MethodProcedure {
    // take the shared fields
    const shared = shapes.scope.extend({
        ...shapes.created,
        ...shapes.parent,
        ...shapes.replay,
        ...shapes.written(options.fields, false),
        ...(options.isInternal === true ? shapes.callers : {}),
        ...fields(options.input),
    });

    return {
        route: { method: "POST", path: name === "create" ? "" : `/${kebabCase(name)}` },
        input:
            options.variants === undefined
                ? shared
                : variantInput(name, shapes, shared, options.variants),
        output: shapes.row,
    };
}

/** Join each variant's columns to a creation's shared fields under its discriminating value. */
function variantInput(
    name: string,
    shapes: ObjectSchema,
    shared: schema.JsonObject,
    variants: Variants,
): InputSchema {
    // require one field telling the variants apart
    const [sum, ...others] = Object.entries(variants);
    if (sum === undefined || others.length > 0) {
        throw new TypeError(`creation ${name} tells its variants apart by one field`);
    }

    // take each variant's columns, required and non-null
    const [field, byValue] = sum;
    const objects = Object.entries(byValue).map(([value, written]) =>
        shared.extend({ [field]: schema.literal(value), ...shapes.required(written) }),
    );

    return InputSchema.union(field, objects);
}

/** Require the receiving parent and the create permission of a caller, over grants read once. */
async function authorizeCreate(call: Call<ObjectTable>, grants?: TupleReader): Promise<void> {
    // require the receiving parent and the creation
    const reader = grants ?? (await call.requireAuthorization().reader(call.scope));
    const parent = call.object.parent && call.parent();
    if (parent) {
        await call.requireReceiving(parent, reader);
    }
    await requireCreatable(call, reader, creationOf(call));
}

/** Require each item's receiving parent and create permission, reading the grants once. */
async function authorizeItems(call: Call<ObjectTable>): Promise<void> {
    const reader = await call.requireAuthorization().reader(call.scope);
    await eachItem(call, (item) => authorizeCreate(item, reader));
}

/** Insert a created object under an unused identifier and require guarded writes. */
async function insertAuthorized(
    call: Call<ObjectTable>,
    insert: (created: Call<ObjectTable>) => Promise<unknown>,
): Promise<{ readonly id: string; readonly row: unknown }> {
    const { id, created, row } = await insertCreated(call, insert);
    await call.requireAuthorization().requireWritable(created, id);

    return { id, row };
}

/** Create one object per item as its own creation does, returning their identifiers in order. */
async function createItems(call: Call<ObjectTable>): Promise<{ ids: string[] }> {
    // create independent items in one write and require each to be writable
    const { object } = call;
    const isSequential =
        call.isPredicted || object.ordering !== undefined || object.lifecycle.versioned;
    if (!isSequential) {
        const ids = await createTogether(call);
        await eachItem(call, (item, index) =>
            call.requireAuthorization().requireWritable(item, aligned(ids, index)),
        );

        return { ids };
    }

    // create one item at a time otherwise
    const ids = await eachItem(call, async (item) => {
        // insert a prediction under the client's identifier
        if (item.isPredicted) {
            if (item.id === undefined) {
                throw new TypeError(`predicted ${item.object.name} creations need an identifier`);
            }
            await createObject(item);

            return item.id;
        }
        // insert under an unused identifier and record its access
        else {
            const { id } = await insertAuthorized(item, createObject);

            return id;
        }
    });

    return { ids };
}

/** Insert every item's row in one write, retrying one item at a time on a conflict. */
async function createTogether(call: Call<ObjectTable>): Promise<string[]> {
    // build each item's row under an unused identifier
    const created = await eachItem(call, async (item) => {
        const id = await createdId(item);
        const row = createdRow(call.object.table, await createdValues(item.with({ id })));

        return { item: item.with({ id }), id, row };
    });

    // write the rows and record each creation's access within one savepoint
    try {
        await call.requireTransaction().savepoint(async () => {
            const table = call.object.table;
            await call.write(...created.map(({ row }) => ({ table, before: null, after: row })));
            for (const { item } of created) {
                await recordCreation(item);
            }
        });
    } catch (error) {
        // find the failing item of a conflict by creating one at a time
        const isConflict =
            (isCallError(error) && error.code === "CONFLICT") ||
            (error instanceof DatabaseError && error.code === "DUPLICATE");
        if (!isConflict) {
            throw error;
        }

        return eachItem(call, async (item) => (await insertCreated(item, createObject)).id);
    }

    return created.map(({ id }) => id);
}

/** Run a call per item of a bulk creation in order, prefixing a failure with its index. */
async function eachItem<Result>(
    call: Call<ObjectTable>,
    run: (item: Call<ObjectTable>, index: number) => Promise<Result>,
): Promise<Result[]> {
    const results: Result[] = [];
    for (const [index, { id, ...written }] of BatchItems.parse(call.input["items"]).entries()) {
        // run the item as its own creation's call
        const item = call.with({
            input: written,
            ...(id === undefined ? {} : { id: schema.string().parse(id) }),
        });
        try {
            results.push(await run(item, index));
        } catch (error) {
            throw itemFailure(call.object, index, error);
        }
    }

    return results;
}

/** Prefix a failing item's call error with its index, mapping a conflict to its fields. */
function itemFailure(object: ObjectType, index: number, error: unknown): unknown {
    // keep other failures as they are
    const failure = object.conflict(error);
    if (!isCallError(failure)) {
        return failure;
    }

    return new CallError(failure.code, {
        status: failure.status,
        message: `items.${index}: ${failure.message}`,
        data: failure.data,
        cause: failure,
    });
}

/** Delete each object a bulk creation created, undoable only by a type with a delete method. */
function deleteCreated(step: Step): readonly sync.Call[] | undefined {
    const { ids } = BatchIds.parse(step.result);
    const calls = ids.flatMap((id) => Step.call(step, "delete", Step.reference(step, id)) ?? []);

    return calls.length === 0 ? undefined : calls;
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

/** Refuse tracked, declarable and versioned objects, whose change records bulk calls skip. */
function requireUnrecorded(object: ObjectType): void {
    if (
        object.lifecycle.tracked !== undefined ||
        object.lifecycle.declarationSchema !== undefined ||
        object.lifecycle.versioned
    ) {
        throw new TypeError(
            `object ${object.name} keeps a record of each change, which many changes at once skip`,
        );
    }
}

/** Refuse guarded fields, whose write permission bulk updates skip. */
function requireUnguarded(object: ObjectType, written: readonly string[]): void {
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
    // decode values
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
    const hasGeneration = Object.hasOwn(definition.columns, "generation");
    const rows = await matchingRows(call);

    // write each match with the next revision and generation
    for (const row of rows) {
        const current: Row = row;
        const updated = Row.parse({
            ...row,
            ...values,
            revision: row.revision + 1,
            ...(hasGeneration
                ? { generation: schema.number().parse(current["generation"]) + 1 }
                : {}),
            updatedAt: call.now,
            updatedBy: call.caller.subject === undefined ? null : Subject.key(call.caller.subject),
        });
        await call.write({ table, before: row, after: definition.values(updated) });
    }

    return { count: rows.length };
}

/** Delete every matching object the caller may delete, each as its deletion does. */
async function deleteMatching(call: Call<ObjectTable>): Promise<{ count: number }> {
    // take each match as its target
    const rows = await matchingRows(call);
    const targets = rows.map((row) => call.with({ id: row.id, target: row }));
    for (const target of targets) {
        requireUnmanaged(target);
    }

    // delete each match on a device as its own deletion
    if (call.isPredicted) {
        for (const target of targets) {
            await deleteObject(target);
        }
    }
    // delete or trash the read matches in one write
    else {
        const table = call.object.table;
        await call.write(...targets.map((target) => ({ table, ...deletion(target) })));
    }

    return { count: rows.length };
}

/** Read a loaded target's deletion: its trashed row where a trash finishes it, else its removal. */
function deletion(target: Call<ObjectTable>): { readonly before: Row; readonly after: Row | null } {
    // remove an object deleting at once
    const before: Row = target.requireTarget();
    const deleting = deletingCall(target);
    if (deleting === undefined) {
        return { before, after: null };
    }

    // trash it at the next revision, recording who asked
    const caller = target.caller.subject;
    const trashed = deleting.revised({
        deletionRequestedAt: target.now,
        deletedBy: caller === undefined ? null : Subject.key(caller),
    });

    return {
        before,
        after: target.object.table[TABLE].values(Row.parse({ ...before, ...trashed })),
    };
}

/** Read the untrashed objects a bulk call matches in its scope, the permitted ones on a server. */
async function matchingRows(call: Call<ObjectTable>) {
    // match the values in the call's scope, leaving out objects whose deletion is requested
    const table = call.object.table;
    const matched = MATCH.parse(call.input["where"]);
    const isDeleting = Object.hasOwn(table[TABLE].columns, "deletionRequestedAt");
    const where = Condition.equal({
        ...matched,
        ...(isDeleting ? { deletionRequestedAt: null } : {}),
        scope: call.scope,
    });

    // read the matches, keeping the permitted ones on the server
    return call.isPredicted
        ? call.read(table).rows(where)
        : call
              .requireAuthorization()
              .keep(await call.requireTransaction().rows(table, where), call.permission());
}

/** Insert a created object. */
async function createObject(call: Call<ObjectTable>): Promise<Select<Table>> {
    // write the created row, then record its access
    const row = createdRow(call.object.table, await createdValues(call));
    await call.write({ table: call.object.table, before: null, after: row });
    await recordCreation(call);

    return row;
}

/** Record a created object's access: a new scope's chain and its first relationships. */
async function recordCreation(call: Call<ObjectTable>): Promise<void> {
    const creating = creationOf(call);
    const isScope = call.object.policy.definition.scope === true;
    if (!call.isPredicted && (isScope || creating !== undefined)) {
        const object = call.object.reference(call.scope, call.requireId());
        await call.requireAuthorization().create(object, (await creating?.(call, object)) ?? {});
    }
}

/** Build the columns a created object is inserted with. */
async function createdValues(call: Call<ObjectTable>): Promise<InsertValue<Table>> {
    // fill caller fields with a calling principal of the declared kinds
    const { object } = call;
    const caller = call.caller.subject;
    const isInternal = call.method.isInternal === true;
    const callers: Record<string, string> = {};
    for (const [name, declared] of Object.entries(object.fields)) {
        const kinds = declared.principals;
        if (!declared.isCallerFilled || isInternal) {
            continue;
        } else if (
            caller === undefined ||
            !(kinds === undefined ? isPrincipal(caller) : kinds.some((kind) => kind.is(caller)))
        ) {
            const named = kinds?.map((kind) => kind.name).join(" or ") ?? "principal";
            throw new CallError("FORBIDDEN", {
                message: `${object.name} is created by a ${named}`,
            });
        }
        callers[name] = declared.type === "subject" ? Subject.key(caller) : caller.id;
    }

    // number versions within their parent
    const version = object.lifecycle.versioned
        ? await nextVersion(object, call.requireParent().id, call)
        : undefined;

    // place an ordered object after its last sibling unless the caller placed it
    const decoded = object.table[TABLE].decode(call.input);
    const parent = object.parent === undefined ? {} : call.parentColumns();
    const position =
        object.ordering === undefined || call.input["position"] !== undefined
            ? undefined
            : await Orderable.next(object, { ...decoded, ...parent, scope: call.scope }, call);

    // require the writer
    if (object.storage === "ephemeral" && call.writerId === undefined) {
        throw new TypeError(`ephemeral ${object.name} objects are created by a writer`);
    }

    // assemble the columns, stamping the times and the author where the table records them
    const author = caller === undefined ? null : Subject.key(caller);
    const columns = object.table[TABLE].columns;
    const isTimed = Object.hasOwn(columns, "createdAt");
    const isAuthored = Object.hasOwn(columns, "createdBy");

    return {
        ...decoded,
        ...parent,
        ...(position === undefined ? {} : { position }),
        id: call.id,
        scope: call.scope,
        ...(version === undefined ? {} : { number: version }),
        ...(object.storage === "ephemeral" ? { writerId: call.writerId } : {}),
        ...callers,
        ...(isTimed ? { createdAt: call.now, updatedAt: call.now } : {}),
        ...(isAuthored ? { createdBy: author, updatedBy: author } : {}),
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
        return await call.requireTransaction().savepoint(async () => {
            // insert under an unused identifier
            const id = await createdId(call);
            const created = call.with({ id });

            return { id, created, row: await insert(created) };
        });
    } catch (error) {
        // report conflicts only to admitted callers
        const isConflict =
            (isCallError(error) && error.code === "CONFLICT") ||
            (error instanceof DatabaseError &&
                (error.code === "DUPLICATE" || error.code === "BROKEN_REFERENCE"));
        if (isConflict) {
            await requireCreatable(call);
        }
        throw error;
    }
}

/** Build a created object's complete row from its written values and the columns' defaults. */
function createdRow(table: Table, values: InsertValue<Table>): Row {
    return table[TABLE].complete(table[TABLE].values(Row.parse(values)));
}

/** Require the create permission on the object as written. */
async function requireCreatable(
    call: Call<ObjectTable>,
    reader?: TupleReader,
    creation?: (call: Call, object: ObjectReference) => Creation | Promise<Creation>,
): Promise<void> {
    // build the row as it will be written, with column defaults
    const id = call.id ?? call.object.generateId();
    const definition = call.object.table[TABLE];
    const values = await createdValues(call.with({ id }));
    const row = definition.defaults(definition.values(Row.parse(values)));

    // grant the creation's first relationships, then check the permission on the row
    const authorization = call.requireAuthorization();
    const grants = reader ?? (await authorization.reader(call.scope));
    const created = call.object.reference(call.scope, id);
    const initial = (await creation?.(call.with({ id }), created)) ?? {};
    const { authorizer } = authorization;
    grants.creating(created, authorizer.initialRelationships(created, initial, call.now));
    await requirePermitted(call, row, grants);
}

/** Read whether a custom method's target declares it scoped: "none". */
function isScopedTarget<Target extends CustomTarget>(
    target: Target | undefined,
): Target extends "none" ? true : false;
/**
 * Read whether a target is none.
 *
 * @construct the result is true exactly when the target is "none", as the signature reads.
 */
function isScopedTarget(target: CustomTarget | undefined): boolean {
    return target === "none";
}

/** Require a scoped method's permission in the call's scope, as on a new object there. */
async function authorizeScoped(
    call: Call<ObjectTable>,
    selected: ((input: Readonly<Record<string, unknown>>) => Row) | undefined,
): Promise<void> {
    // read the selected columns, else the input's, and a nested type's parent
    const definition = call.object.table[TABLE];
    const input: Readonly<Record<string, unknown>> = call.input;
    const named =
        selected?.(input) ??
        Object.fromEntries(
            Object.keys(definition.columns)
                .filter((column) => input[column] !== undefined)
                .map((column) => [column, input[column]]),
        );
    const parent = call.parentColumns();

    // check the permission on a new object with those columns, under an unused key for a keyed type
    const id = call.object.keyed === undefined ? call.object.generateId() : crypto.randomUUID();
    const row = definition.defaults({ ...named, id, scope: call.scope, ...parent });
    await requirePermitted(call, row, await call.requireAuthorization().reader(call.scope));
}

/** Require the call's permission on a row, challenging for step-up authentication. */
async function requirePermitted(
    call: Call<ObjectTable>,
    row: Row,
    grants: TupleReader,
): Promise<void> {
    // check the permission on the row
    const authorization = call.requireAuthorization();
    const { authorizer, snapshot } = authorization;
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

    // check whether the caller reads the object, which a scoped call names none of
    const reading = call.object.reading;
    const isReadable =
        call.method.isScoped === true ||
        (reading !== undefined &&
            (await authorizer.checkRows(snapshot, reading, access, [row])).permitted.has(0));

    // refuse a reader, and hide the object from others
    const denial = new CallError("FORBIDDEN", {
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
        throw new CallError("CONFLICT", { message: `${object.name} is already deleted` });
    }

    // request deletion when a trash or controller finishes it
    const deleting = deletingCall(call);
    if (deleting !== undefined) {
        await deleting.update({
            deletionRequestedAt: call.now,
            deletedBy: call.caller.subject === undefined ? null : Subject.key(call.caller.subject),
        });

        return {};
    }

    // delete at the loaded revision
    await call.remove();

    return {};
}

/** Read a call by its table's deletion columns, absent for an object deleting at once. */
function deletingCall(
    call: Call<ObjectTable>,
): Call<TraitTable<{ readonly recoverable: RecoverableDefinition }>> | undefined;
/**
 * Read a call by its table's deletion columns, for objects a trash or controller deletes.
 *
 * @construct defineObject derives deletion columns for every `recoverable` or `controlled` object.
 */
function deletingCall(call: Call<ObjectTable>): Call | undefined {
    return call.object.lifecycle.recoverable === undefined && !call.object.isControlled
        ? undefined
        : call;
}

/** Accept a caller's unused identifier, or generate one: random in a prediction, else derived. */
async function createdId(call: Call<ObjectTable>): Promise<string> {
    // generate a missing identifier
    const { object, id: chosen } = call;
    if (chosen === undefined) {
        return call.isPredicted ? object.generateId() : call.generateId(object);
    }

    // find a row with the identifier, or a relationship a deleted object left under it
    const { packageId, name } = object.policy.definition;
    const id = schema.anyIdentifier().parse(chosen);
    const rows = await call.requireTransaction().images(object.table, { id });
    const relationships = await call.requireAuthorization().transaction.images(accessRelationship, {
        packageId,
        type: name,
        objectId: id,
    });
    const isTaken = rows.length > 0 || relationships.length > 0;

    // refuse a taken identifier
    if (isTaken) {
        throw new CallError("CONFLICT", { message: `${object.name} identifier is taken` });
    }

    return chosen;
}

/** Read a page of a query's rows or its aggregate groups. */
export async function listObjects(call: Call) {
    // read an events type's rows from the event log on a server, hot and flushed alike
    const input = LIST_INPUT.parse(call.input);
    if (call.object.storage === "events" && !call.isPredicted) {
        return EventListing.page(call, input);
    }

    // bind the cursor to the query
    const { cursor, limit, at: _at, branch: _branch, ...options } = input;
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
    const view = call.view();
    await dataflow.load(await View.of(view.database, call.snapshot), after);
    if (node.aggregate !== undefined && position !== undefined) {
        throw new CallError("BAD_REQUEST", {
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
            database: call.view().database,
            changesThrough: changesThroughLog(call.view().database),
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
    if (isManaging(call.target)) {
        throw new CallError("MANAGED", {
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
        schema.Object<
            ReturnType<typeof ListPage.schema<RowSchema<Object>>>["shape"] & typeof ListShape
        >
    >;
    create: Procedure<
        IsSum<Declared["variants"]> extends true
            ? VariantSchema<Object, Declared, CreateShape<Object, Declared>>
            : schema.Object<CreateShape<Object, Declared>>,
        RowSchema<Object>
    >;
    createMany: Procedure<
        schema.Object<
            ScopeShape<Object> & ReplayShape<Object> & { items: ItemsSchema<Object, Declared> }
        >,
        schema.Object<{ ids: schema.Schema<string[]> }>
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
                ReplayShape<Object> & {
                    where: schema.Object<MatchShape<Object, Declared>>;
                } & WrittenShape<Object, Declared, true>
        >,
        schema.Object<{ count: schema.Schema<number> }>
    >;
    delete: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & RevisionField>,
        schema.Object<{}>
    >;
    deleteMany: Procedure<
        schema.Object<
            ScopeShape<Object> &
                ReplayShape<Object> & { where: schema.Object<MatchShape<Object, Declared>> }
        >,
        schema.Object<{ count: schema.Schema<number> }>
    >;
    custom: Procedure<
        CustomSchema<
            (NonNullable<Declared["isScoped"]> extends true
                ? ScopeShape<Object>
                : TargetShape<Object>) &
                (Declared["mutates"] extends false ? {} : ReplayShape<Object>),
            Declared["input"]
        >,
        [NonNullable<Declared["output"]>] extends [never]
            ? RowSchema<Object>
            : NonNullable<Declared["output"]>
    >;
};

/** The fields of every creation: scope, chosen identifier, parent, replay, fields and input. */
type CreateShape<Object extends ObjectType, Declared extends Method> = ScopeShape<Object> &
    CreationShape<Object> &
    ParentShape<Object> &
    ReplayShape<Object> &
    WrittenShape<Object, Declared, false> &
    FieldShape<Declared>;

/** The items of a bulk creation, each a creation's chosen identifier, parent and fields. */
type ItemsSchema<
    Object extends ObjectType,
    Declared extends Method,
    Item extends schema.Schema = schema.Object<
        CreationShape<Object> & ParentShape<Object> & WrittenShape<Object, Declared, false>
    >,
> = schema.Schema<schema.Output<Item>[], schema.Input<Item>[]>;

/** Whether a method declares the column sets of a sum type. */
export type IsSum<Sum> = [NonNullable<Sum>] extends [never]
    ? false
    : object extends NonNullable<Sum>
      ? false
      : true;

/** The written values a call matches objects by, in their JSON form. */
type MatchShape<Object extends ObjectType, Declared extends Method> = ColumnShape<
    Object,
    string extends NonNullable<Declared["match"]>[number]
        ? WrittenField<Object["fields"]>
        : NonNullable<Declared["match"]>[number],
    true
>;

/** The identifier a caller may choose for a created object. */
type CreationShape<Object extends ObjectType> = {
    id: schema.ExactOptional<IdSchema<Object>>;
};

/** Read how a call creates its object's first relationships. */
function creationOf(
    call: Call,
): ((call: Call, object: ObjectReference) => Creation | Promise<Creation>) | undefined {
    const isOwnedScope =
        call.object.roles !== undefined && call.object.policy.definition.scope === true;

    return call.method.creation ?? (isOwnedScope ? relateCreator("owner") : undefined);
}

/** Relate the calling principal to a new object, and a new scope's owner role to its holders. */
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

/** Declare methods with `method.query(...)`, `method.mutation(...)`, `method.get(...)` and more. */
export const method: MethodBuilder = {
    query,
    mutation,
    get,
    list,
    create,
    createMany,
    update,
    updateMany,
    delete: remove,
    deleteMany,
};

import { present, type JsonObject, schema } from "@destack/schema";
import { type Expression, Row, Table } from "@destack/db";

import { MethodContext } from "./context.ts";
import { Step } from "./step.ts";
import type * as sync from "@destack/sync";
import {
    defineMethod,
    Method,
    type MethodDefinition,
    type MethodSource,
    type MethodVisibility,
    type MethodConflict,
} from "./method.ts";
import {
    Empty,
    PageFields,
    RevisionFields,
    type CustomSchema,
    type InputFields,
    type ColumnFields,
    type WrittenFields,
    type IdSchema,
    type ParentFields,
    type Procedure,
    type ReplayFields,
    type RowSchema,
    type ScopeFields,
    type TargetFields,
    type VariantSchema,
    ListPage,
} from "./procedure.ts";
import { type InputObject, InputSchema } from "./input.ts";
import { Casing } from "../object/name.ts";
import { ListFields, QueryFields, ViewFields } from "../replica/replica.ts";
import type { ObjectType, CreatedField } from "../object/object.ts";

import type { StoreExecution } from "./execution.ts";

import { BatchIds, ObjectRecord, TagFields } from "./record.ts";

import type { MethodLimits } from "./limits.ts";
import { Transitions } from "../trait/transition.ts";
import type { Message } from "@destack/locale";

/** The most items one bulk creation takes, as many as one list page reads. */
export const BATCH_LIMIT = 1000;

/** The count of objects a bulk update or deletion changed. */
export const BatchPayload = schema.object({ count: schema.number().int().nonnegative() });

/** The column sets of a sum type, by discriminating field and then value. */
export type Variants<Fields extends string = string> = Readonly<
    Record<string, Readonly<Record<string, readonly Fields[]>>>
>;

/** How a creation method creates: its written fields, input, creator and prediction. */
export interface CreateOptions<
    Fields extends string,
    Input extends InputObject,
    Prepared extends schema.Schema,
    Visibility extends MethodVisibility,
    Sum extends Variants = Variants,
    Filled extends string = string,
    Created extends string = string,
> {
    /** Who may call the creation; server-only creations take the caller fields as input. */
    readonly visibility?: Visibility;
    /** The fields the caller writes, every written field when absent. */
    readonly fields?: readonly Fields[];
    /** The written fields the handler fills where the caller leaves them out. */
    readonly filled?: readonly Filled[];
    /** The column sets of a sum-typed row, of which the caller writes one beside the fields. */
    readonly variants?: Sum;
    /** The caller-supplied values beyond fields. */
    readonly input?: Input;
    /** The external work the creation prepares before its transaction. */
    readonly prepared?: Prepared;
    /** The relation the calling principal takes to the new object. */
    readonly creator?: string;
    /** Whether clients predict the creation. */
    readonly isPredicted?: boolean;
    /** The input fields each release computes from earlier input, by introducing release. */
    readonly convert?: Readonly<Record<number, Readonly<Record<string, Expression>>>>;
    /** How often the method runs per key, and whose limit a call counts against. */
    readonly limits?: MethodLimits;
    /** What a creation answers when the row it writes matches an existing one on a unique key. */
    readonly onConflict?: MethodConflict<NoInfer<Created>>;
}

/** How a bulk creation creates: its written fields and creator. */
export interface CreateManyOptions<Fields extends string, Visibility extends MethodVisibility> {
    /** Who may call the creation; server-only creations take the caller fields as input. */
    readonly visibility?: Visibility;
    /** The fields the caller writes on each item. */
    readonly fields: readonly Fields[];
    /** The relation the calling principal takes to each new object. */
    readonly creator?: string;
}

/** How an update method updates: its written fields, input and prediction. */
export interface UpdateOptions<
    Fields extends string,
    Input extends InputObject,
    Prepared extends schema.Schema,
    Visibility extends MethodVisibility = MethodVisibility,
> {
    /** Who may call the update. */
    readonly visibility?: Visibility;
    /** The fields the caller writes, every written field when absent. */
    readonly fields?: readonly Fields[];
    /** The caller-supplied values beyond fields. */
    readonly input?: Input;
    /** The external work the update prepares before its transaction. */
    readonly prepared?: Prepared;
    /** Whether clients predict the update. */
    readonly isPredicted?: boolean;
    /** The input fields each release computes from earlier input, by introducing release. */
    readonly convert?: Readonly<Record<number, Readonly<Record<string, Expression>>>>;
    /** How often the method runs per key, and whose limit a call counts against. */
    readonly limits?: MethodLimits;
}

/** How a bulk update matches and writes objects. */
export interface UpdateManyOptions<
    Fields extends string,
    Match extends string,
    Visibility extends MethodVisibility = MethodVisibility,
> {
    /** Who may call the update. */
    readonly visibility?: Visibility;
    /** The fields the call writes on each matched object. */
    readonly fields: readonly Fields[];
    /** The fields the call matches objects by. */
    readonly match: readonly Match[];
    /** The input fields each release computes from earlier input, by introducing release. */
    readonly convert?: Readonly<Record<number, Readonly<Record<string, Expression>>>>;
    /** How often the method runs per key, and whose limit a call counts against. */
    readonly limits?: MethodLimits;
}

/** How a bulk deletion matches objects. */
export interface DeleteManyOptions<Match extends string> {
    /** The fields the call matches objects by. */
    readonly match: readonly Match[];
}

/** How a deletion method deletes. */
export interface RemoveOptions<
    Prepared extends schema.Schema,
    Visibility extends MethodVisibility = MethodVisibility,
> {
    /** Who may call the deletion. */
    readonly visibility?: Visibility;
    /** The external work the deletion prepares before its transaction. */
    readonly prepared?: Prepared;
    /** Whether clients predict the deletion. */
    readonly isPredicted?: boolean;
}

/** Describe a bulk update of matching objects, or of listed objects at their revisions. */
function bulkUpdate<
    Permission extends string,
    Fields extends string,
    Match extends string,
    Itemized extends boolean,
    Visibility extends MethodVisibility,
>(
    permission: Permission,
    options: Omit<UpdateManyOptions<Fields, Match, Visibility>, "match">,
    match: readonly Match[] | undefined,
    isItemized: Itemized,
    source: MethodSource,
): MethodDefinition<{
    kind: "updateMany";
    permission: Permission;
    output: typeof BatchPayload;
    mutates: true;
    match: Match;
    fields: Fields;
    items: Itemized;
    target: "scope";
    visibility: Visibility;
}> {
    return {
        kind: "updateMany",
        source,
        permission,
        output: BatchPayload,
        mutates: true,
        ...(options.visibility === undefined ? {} : { visibility: options.visibility }),
        ...(isItemized ? { isItemized } : {}),
        ...(options.convert === undefined ? {} : { convert: options.convert }),
        ...(options.limits === undefined ? {} : { limits: options.limits }),
        target: "scope",
        result: "value",
        procedure: (name, schemas) => ({
            route: { method: "PATCH", path: name === "updateMany" ? "" : `/${Casing.kebab(name)}` },
            input: schemas.scope.extend(
                match === undefined
                    ? {
                          ...schemas.replay,
                          items: schema
                              .array(
                                  schema.object({
                                      id: schema.string(),
                                      revision: schema.number().int().positive(),
                                      ...schemas.written(options.fields, true),
                                  }),
                              )
                              .max(BATCH_LIMIT),
                      }
                    : {
                          ...schemas.replay,
                          where: schema.object(schemas.written(match, true)),
                          ...schemas.written(options.fields, true),
                      },
            ),
            output: BatchPayload,
        }),
        validate: (object) => {
            // refuse fields an update of many objects cannot write alike
            ObjectRecord.requireUnrecorded(object);
            ObjectRecord.requireUnguarded(object, options.fields);
            ObjectRecord.requireUnnumbered(object, options.fields);
            ObjectRecord.requireUnmoved(object, options.fields);
        },
        handler: (context, execution) =>
            match === undefined
                ? execution.store.record.updateItems(context, execution, options.fields)
                : execution.store.record.updateMatching(context, execution, options.fields),
    };
}

/** What a method acts on: its loaded object, its object by identifier, or its scope. */
export type MethodTarget = "object" | "id" | "scope";

/** A custom method's permission, input, output and inverse. */
export interface CustomDefinition<
    Permission extends string,
    Input extends InputSchema,
    Output extends schema.Schema,
    Prepared extends schema.Schema,
    Visibility extends MethodVisibility = "client",
    Target extends MethodTarget = "object",
> {
    /** The action as people read it, such as in an audit log. */
    readonly title: Message;
    /** The permission the caller needs on the target. */
    readonly permission: Permission;
    /** Whether the method stays off public routes, run only by the principals' code. */
    readonly visibility?: Visibility;
    /** The caller-supplied fields. */
    readonly input?: Input;
    /** The result, the updated object when absent. */
    readonly output?: Output;
    /** The external work the method prepares before its transaction. */
    readonly prepared?: Prepared;
    /** Whether each reading call is audited. */
    readonly isAudited?: boolean;
    /** The fields of the result an audited read's event records. */
    readonly audit?: { readonly details: schema.Object<Record<string, schema.Schema>> };
    /** What the method acts on: by default its target, loaded under the caller's permission. */
    readonly target?: Target;
    /** The columns of every object a scoped method acts on, else the input's columns. */
    readonly selected?: (input: JsonObject) => Row;
    /** The inverse method name, or a function deriving inverse calls. */
    readonly inverse?: string | ((step: Step) => readonly sync.Call[] | undefined);
    /** Compose a step into the previous one as one call doing both, absent when apart. */
    readonly compose?: (previous: Step, next: Step) => Step | undefined;
    /** The input fields each release computes from earlier input, by introducing release. */
    readonly convert?: Readonly<Record<number, Readonly<Record<string, Expression>>>>;
    /** How often the method runs per key, and whose limit a call counts against. */
    readonly limits?: MethodLimits;
    /** Whether clients predict the method once handled. */
    readonly isPredicted?: boolean;
}

/** The procedure each record method derives. */
export type RecordProcedures<Object extends ObjectType, Declaration extends Method> = {
    get: Procedure<schema.Object<TargetFields<Object> & typeof ViewFields>, RowSchema<Object>>;
    list: Procedure<
        schema.Object<
            ScopeFields<Object> & typeof QueryFields & typeof PageFields & typeof ViewFields
        >,
        schema.Object<
            ReturnType<typeof ListPage.schema<RowSchema<Object>>>["shape"] & typeof ListFields
        >
    >;
    create: Procedure<
        IsSum<Declaration["variants"]> extends true
            ? VariantSchema<Object, Declaration, CreateFields<Object, Declaration>>
            : schema.Object<CreateFields<Object, Declaration>>,
        RowSchema<Object>
    >;
    createMany: Procedure<
        schema.Object<
            ScopeFields<Object> & ReplayFields<Object> & { items: ItemsSchema<Object, Declaration> }
        >,
        schema.Object<{ ids: schema.Schema<string[]> }>
    >;
    update: Procedure<
        schema.Object<
            TargetFields<Object> &
                ReplayFields<Object> &
                typeof RevisionFields &
                WrittenFields<Object, Declaration, true> &
                typeof TagFields &
                InputFields<Declaration>
        >,
        RowSchema<Object>
    >;
    updateMany: Procedure<
        schema.Object<
            ScopeFields<Object> &
                ReplayFields<Object> &
                (NonNullable<Declaration["isItemized"]> extends true
                    ? { items: UpdatedItemsSchema<Object, Declaration> }
                    : {
                          where: schema.Object<MatchFields<Object, Declaration>>;
                      } & WrittenFields<Object, Declaration, true>)
        >,
        schema.Object<{ count: schema.Schema<number> }>
    >;
    delete: Procedure<
        schema.Object<TargetFields<Object> & ReplayFields<Object> & typeof RevisionFields>,
        schema.Object<{}>
    >;
    deleteMany: Procedure<
        schema.Object<
            ScopeFields<Object> &
                ReplayFields<Object> & { where: schema.Object<MatchFields<Object, Declaration>> }
        >,
        schema.Object<{ count: schema.Schema<number> }>
    >;
    custom: Procedure<
        CustomSchema<
            (NonNullable<Declaration["target"]> extends "scope"
                ? ScopeFields<Object>
                : TargetFields<Object>) &
                (Declaration["mutates"] extends false ? {} : ReplayFields<Object>),
            Declaration["input"]
        >,
        [NonNullable<Declaration["output"]>] extends [never]
            ? RowSchema<Object>
            : NonNullable<Declaration["output"]>
    >;
};

/** The fields of every creation: scope, chosen identifier, parent, replay, fields and input. */
type CreateFields<Object extends ObjectType, Declaration extends Method> = ScopeFields<Object> &
    CreationFields<Object> &
    ParentFields<Object> &
    ReplayFields<Object> &
    WrittenFields<Object, Declaration, false> &
    typeof TagFields &
    InputFields<Declaration>;

/** The items of a bulk creation, each a creation's chosen identifier, parent and fields. */
type ItemsSchema<
    Object extends ObjectType,
    Declaration extends Method,
    Item extends schema.Schema = schema.Object<
        CreationFields<Object> & ParentFields<Object> & WrittenFields<Object, Declaration, false>
    >,
> = schema.Schema<schema.Output<Item>[], schema.Input<Item>[]>;

/** The items of a bulk update: each object's identifier and revision with its written fields. */
type UpdatedItemsSchema<
    Object extends ObjectType,
    Declaration extends Method,
    Item extends schema.Schema = schema.Object<
        {
            id: schema.Schema<string, string>;
            revision: schema.Schema<number, number>;
        } & WrittenFields<Object, Declaration, true>
    >,
> = schema.Schema<schema.Output<Item>[], schema.Input<Item>[]>;

/** Whether a method declares the column sets of a sum type. */
export type IsSum<Sum> = [NonNullable<Sum>] extends [never]
    ? false
    : object extends NonNullable<Sum>
      ? false
      : true;

/** The written values a call matches objects by, in their JSON form. */
type MatchFields<Object extends ObjectType, Declaration extends Method> = ColumnFields<
    Object,
    string extends NonNullable<Declaration["match"]>[number]
        ? CreatedField<Object["fields"]>
        : NonNullable<Declaration["match"]>[number],
    true
>;

/** The identifier a caller may choose for a created object, and an event's identity and time. */
type CreationFields<Object extends ObjectType> = Object["storage"] extends "events"
    ? { id: IdSchema<Object>; time: schema.Schema<number, number> }
    : { id: schema.ExactOptional<IdSchema<Object>> };

/** The method declarations of one object type, typed by its table, fields and permissions. */
export class MethodBuilder<
    Definition extends Table = Table,
    Created extends string = string,
    Granted extends string = string,
    Updated extends string = string,
> {
    /** Read one object. */
    get<const Permission extends Granted>(permission: Permission) {
        return defineMethod<{
            kind: "get";
            target: "object";
            permission: Permission;
            mutates: false;
            table: Definition;
        }>({
            kind: "get",
            source: { builder: "get", arguments: [permission] },
            permission,
            mutates: false,
            target: "object",
            result: "object",
            procedure: (_name, schemas) => ({
                route: { method: "GET", path: "/{id}" },
                input: schemas.target.extend(ViewFields),
                output: schemas.row,
            }),
            handler: async (context) => context.target,
        });
    }

    /** Read a page of a scope's objects or aggregate groups. */
    list<const Permission extends Granted>(permission: Permission) {
        return defineMethod<{
            kind: "list";
            permission: Permission;
            mutates: false;
            table: Definition;
            target: "scope";
        }>({
            kind: "list",
            source: { builder: "list", arguments: [permission] },
            permission,
            mutates: false,
            target: "scope",
            result: "page",
            procedure: (_name, schemas) => ({
                route: { method: "POST", path: "/query" },
                input: schemas.scope.extend({ ...QueryFields, ...PageFields, ...ViewFields }),
                output: ListPage.schema(schemas.row).extend(ListFields),
            }),
            handler: (context, execution) => execution.store.record.list(context, execution),
        });
    }

    /** Create an object from the fields the caller writes. */
    create<
        const Permission extends Granted,
        const Fields extends Created = Created,
        Input extends InputObject = never,
        Prepared extends schema.Schema = never,
        const Visibility extends MethodVisibility = "client",
        const Sum extends Variants = never,
        const Filled extends Fields = never,
    >(
        this: MethodBuilder<Definition>,
        permission: Permission,
        options: CreateOptions<Fields, Input, Prepared, Visibility, Sum, Filled, Created> = {},
    ) {
        // relate the creation's creator, and answer the object its unique key matches on a conflict
        const { creator, onConflict, ...declared } = options;

        return defineMethod<{
            kind: "create";
            permission: Permission;
            input: NoInfer<Input>;
            prepared: NoInfer<Prepared>;
            mutates: true;
            fields: NoInfer<Fields>;
            filled: NoInfer<Filled>;
            variants: NoInfer<Sum>;
            table: Definition;
            visibility: NoInfer<Visibility>;
            target: "scope";
        }>({
            kind: "create",
            source: { builder: "create", arguments: [permission, options] },
            permission,
            mutates: true,
            ...declared,
            target: "scope",
            result: "object",
            procedure: (name, schemas) => ObjectRecord.createProcedure(name, schemas, options),
            ...(creator === undefined ? {} : { relate: ObjectRecord.relateCreator(creator) }),
            ...(onConflict === undefined ? {} : { onConflict }),
            handler: (context, execution) =>
                execution.store.record.createObject(context, execution, onConflict),
            authorize: (context, execution) =>
                execution.store.record.authorizeCreate(context, execution),
            inverse: (step) => {
                // restore an existing object a conflict updated, else delete the created one
                if (step.before !== undefined) {
                    return ObjectRecord.restoreConflict(step, onConflict?.update ?? []);
                }
                const call = Step.call(step, "delete", Step.target(step));

                return call === undefined ? undefined : [call];
            },
            async execute(context, execution) {
                // predict the creation under the identifier its call carries, its home recording its access
                if (execution.isPredicted) {
                    present(context.id, `the identifier of a predicted ${context.object.name}`);

                    return Method.handle(this, context, execution);
                }

                // append an events type's object to the event log
                if (context.object.storage === "events") {
                    const row = ObjectRecord.createdRow(
                        context.object.table,
                        await execution.store.record.createdValues(context, execution),
                    );
                    await execution.store.record.append(context, execution, [row]);

                    return row;
                }

                // insert under an unused identifier and record its access
                const { row } = await execution.store.record.insertAuthorized(
                    context,
                    execution,
                    (inserted) => Method.handle(this, inserted, execution),
                );

                return row;
            },
        });
    }

    /** Create one object per item from the fields the caller writes. */
    createMany<
        const Permission extends Granted,
        const Fields extends Created = Created,
        const Visibility extends MethodVisibility = "client",
    >(permission: Permission, options: CreateManyOptions<Fields, Visibility>) {
        // relate the creation's creator
        const { creator, ...declared } = options;

        return defineMethod<{
            kind: "createMany";
            permission: Permission;
            output: typeof BatchIds;
            mutates: true;
            fields: NoInfer<Fields>;
            table: Definition;
            visibility: NoInfer<Visibility>;
            target: "scope";
        }>({
            kind: "createMany",
            source: { builder: "createMany", arguments: [permission, options] },
            permission,
            output: BatchIds,
            mutates: true,
            ...declared,
            target: "scope",
            result: "value",
            procedure: (name, schemas) => ({
                route: { method: "POST", path: `/${Casing.kebab(name)}` },
                input: schemas.scope.extend({
                    ...schemas.replay,
                    items: schema
                        .array(
                            schema.object({
                                ...schemas.created,
                                ...schemas.parent,
                                ...schemas.written(options.fields, false),
                                ...(options.visibility !== undefined &&
                                options.visibility !== "client"
                                    ? schemas.callers
                                    : {}),
                            }),
                        )
                        .max(BATCH_LIMIT),
                }),
                output: BatchIds,
            }),
            validate: (object) => ObjectRecord.requireUnrecorded(object),
            ...(creator === undefined ? {} : { relate: ObjectRecord.relateCreator(creator) }),
            handler: (context, execution) => execution.store.record.createItems(context, execution),
            authorize: (context, execution) =>
                execution.store.record.authorizeItems(context, execution),
            inverse: (step) => ObjectRecord.deleteCreated(step),
        });
    }

    /** Update an object's fields written after creation, its states through their transitions. */
    update<
        const Permission extends Granted,
        const Fields extends Updated = Updated,
        Input extends InputObject = never,
        Prepared extends schema.Schema = never,
        const Visibility extends MethodVisibility = "client",
    >(permission: Permission, options: UpdateOptions<Fields, Input, Prepared, Visibility> = {}) {
        return defineMethod<{
            kind: "update";
            target: "object";
            permission: Permission;
            input: NoInfer<Input>;
            prepared: NoInfer<Prepared>;
            mutates: true;
            fields: NoInfer<Fields>;
            table: Definition;
            visibility: NoInfer<Visibility>;
        }>({
            kind: "update",
            source: { builder: "update", arguments: [permission, options] },
            permission,
            mutates: true,
            ...options,
            target: "object",
            result: "object",
            procedure: (_name, schemas) => ({
                route: { method: "PATCH", path: "/{id}" },
                input: schemas.target.extend({
                    ...schemas.replay,
                    ...RevisionFields,
                    ...schemas.written(options.fields, true),
                    ...TagFields,
                    ...ObjectRecord.fields(options.input),
                }),
                output: schemas.row,
            }),
            handler: (context) => context.update(Table.decode(context.object.table, context.input)),
            inverse: (step) => ObjectRecord.restoreUpdated(step),
            compose: (previous, next) => Step.compose(previous, next),
            async execute(context, execution) {
                // refuse changing managed fields and unwritable fields
                const changed = Object.keys(context.input).filter((name) =>
                    Object.hasOwn(context.object.fields, name),
                );
                execution.store.record.requireUnmanagedFields(context, execution, changed);
                if (!execution.isPredicted) {
                    await execution.authorization.requireWritable(context, context.id, execution);
                }

                // move each state the call changes only as a transition between the two states does
                const moved = changed.filter(
                    (name) =>
                        context.object.fields[name]?.machine !== undefined &&
                        context.input[name] !== context.target[name],
                );
                for (const name of moved) {
                    const to = schema.string().parse(context.input[name]);
                    await Transitions.require(context, name, to, execution.isPredicted);
                }

                return Method.handle(this, context, execution);
            },
        });
    }

    /** Update every matching object the caller may change, returning the count. */
    updateMany<
        const Permission extends Granted,
        const Fields extends Updated = Updated,
        const Match extends string = string,
        const Visibility extends MethodVisibility = "client",
    >(permission: Permission, options: UpdateManyOptions<Fields, Match, Visibility>) {
        return defineMethod<{
            kind: "updateMany";
            permission: Permission;
            output: typeof BatchPayload;
            mutates: true;
            match: NoInfer<Match>;
            fields: NoInfer<Fields>;
            items: false;
            table: Definition;
            target: "scope";
            visibility: NoInfer<Visibility>;
        }>(
            bulkUpdate(permission, options, options.match, false, {
                builder: "updateMany",
                arguments: [permission, options],
            }),
        );
    }

    /** Update each listed object at its revision, all or none, returning the count. */
    updateItems<
        const Permission extends Granted,
        const Fields extends Updated = Updated,
        const Visibility extends MethodVisibility = "client",
    >(
        permission: Permission,
        options: Omit<UpdateManyOptions<Fields, never, Visibility>, "match">,
    ) {
        return defineMethod<{
            kind: "updateMany";
            permission: Permission;
            output: typeof BatchPayload;
            mutates: true;
            match: never;
            items: true;
            fields: NoInfer<Fields>;
            table: Definition;
            target: "scope";
            visibility: NoInfer<Visibility>;
        }>(
            bulkUpdate(permission, options, undefined, true, {
                builder: "updateItems",
                arguments: [permission, options],
            }),
        );
    }
    /** Delete an object. */
    delete<
        const Permission extends Granted,
        Prepared extends schema.Schema = never,
        const Visibility extends MethodVisibility = "client",
    >(permission: Permission, options: RemoveOptions<Prepared, Visibility> = {}) {
        return defineMethod<{
            kind: "delete";
            target: "object";
            permission: Permission;
            output: typeof Empty;
            prepared: NoInfer<Prepared>;
            mutates: true;
            table: Definition;
            visibility: NoInfer<Visibility>;
        }>({
            kind: "delete",
            source: { builder: "delete", arguments: [permission, options] },
            permission,
            output: Empty,
            mutates: true,
            ...options,
            target: "object",
            result: "value",
            procedure: (_name, schemas) => ({
                route: { method: "DELETE", path: "/{id}" },
                input: schemas.target.extend({ ...schemas.replay, ...RevisionFields }),
                output: Empty,
            }),
            handler: (context, execution) => execution.store.record.deleteObject(context),
            inverse: (step) => {
                // restore from the trash
                const call =
                    step.object.lifecycle.recoverable === undefined
                        ? undefined
                        : Step.call(step, "restore", Step.target(step));

                return call === undefined ? undefined : [call];
            },
            async execute(context, execution) {
                // refuse managed objects
                execution.store.record.requireUnmanaged(context, execution);

                return Method.handle(this, context, execution);
            },
        });
    }

    /** Delete every matching object the caller may delete, returning the count. */
    deleteMany<const Permission extends Granted, const Match extends string = string>(
        permission: Permission,
        options: DeleteManyOptions<Match>,
    ) {
        return defineMethod<{
            kind: "deleteMany";
            permission: Permission;
            output: typeof BatchPayload;
            mutates: true;
            match: NoInfer<Match>;
            table: Definition;
            target: "scope";
        }>({
            kind: "deleteMany",
            source: { builder: "deleteMany", arguments: [permission, options] },
            permission,
            output: BatchPayload,
            mutates: true,
            target: "scope",
            result: "value",
            procedure: (name, schemas) => ({
                route: {
                    method: "DELETE",
                    path: name === "deleteMany" ? "" : `/${Casing.kebab(name)}`,
                },
                input: schemas.scope.extend({
                    ...schemas.replay,
                    where: schema.object(schemas.written(options.match, true)),
                }),
                output: BatchPayload,
            }),
            validate: (object) => ObjectRecord.requireUnrecorded(object),
            handler: (context, execution) =>
                execution.store.record.deleteMatching(context, execution),
        });
    }

    /** Declare a method reading one object, as tRPC's `query`. */
    query<
        const Permission extends Granted,
        Input extends InputSchema = never,
        Output extends schema.Schema = never,
        Prepared extends schema.Schema = never,
        const Visibility extends MethodVisibility = "client",
        const Target extends MethodTarget = "object",
    >(definition: CustomDefinition<Permission, Input, Output, Prepared, Visibility, Target>) {
        return defineMethod<{
            kind: "custom";
            permission: Permission;
            input: NoInfer<Input>;
            output: NoInfer<Output>;
            prepared: NoInfer<Prepared>;
            mutates: false;
            table: Definition;
            visibility: NoInfer<Visibility>;
            target: NoInfer<Target>;
        }>(this.#custom(definition, false));
    }

    /** Declare a method changing one object, as tRPC's `mutation`. */
    mutation<
        const Permission extends Granted,
        Input extends InputSchema = never,
        Output extends schema.Schema = never,
        Prepared extends schema.Schema = never,
        const Visibility extends MethodVisibility = "client",
        const Target extends MethodTarget = "object",
    >(definition: CustomDefinition<Permission, Input, Output, Prepared, Visibility, Target>) {
        return defineMethod<{
            kind: "custom";
            permission: Permission;
            input: NoInfer<Input>;
            output: NoInfer<Output>;
            prepared: NoInfer<Prepared>;
            mutates: true;
            table: Definition;
            visibility: NoInfer<Visibility>;
            target: NoInfer<Target>;
        }>(this.#custom(definition, true));
    }

    /** Describe a custom method on one object, reading or changing it. */
    #custom<
        const Permission extends string,
        Input extends InputSchema,
        Output extends schema.Schema,
        Prepared extends schema.Schema,
        const Mutates extends boolean,
        const Visibility extends MethodVisibility,
        const Target extends MethodTarget,
    >(
        definition: CustomDefinition<Permission, Input, Output, Prepared, Visibility, Target>,
        mutates: Mutates,
    ): MethodDefinition<{
        kind: "custom";
        permission: Permission;
        input: Input;
        output: Output;
        prepared: Prepared;
        mutates: Mutates;
        table: Definition;
        visibility: Visibility;
        target: Target;
    }> {
        // act in the scope for a method without a target
        const { inverse, selected, ...declared } = definition;
        const isScoped = definition.target === "scope";

        return {
            kind: "custom",
            source: { builder: mutates ? "mutation" : "query", arguments: [definition] },
            ...declared,
            ...(isScoped
                ? {
                      authorize: (context: MethodContext, execution: StoreExecution) =>
                          execution.store.record.authorizeScoped(context, execution, selected),
                  }
                : {}),
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
            ...(definition.target === undefined ? {} : { target: definition.target }),
            result: definition.output === undefined ? "object" : "value",
            procedure: (name, schemas) => {
                // select the target, else the scope
                const selection = isScoped ? schemas.scope : schemas.target;

                return {
                    route: {
                        method: "POST",
                        path: isScoped ? `/${Casing.kebab(name)}` : `/{id}/${Casing.kebab(name)}`,
                    },
                    input: InputSchema.map(definition.input ?? schema.object({}), (input) =>
                        selection.extend({ ...(mutates ? schemas.replay : {}), ...input.shape }),
                    ),
                    output: definition.output ?? schemas.row,
                };
            },
        };
    }
}

/** The method builder of object types whose table its callers' types state. */
export const method = new MethodBuilder();

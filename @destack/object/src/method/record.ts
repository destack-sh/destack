import { Orderable } from "../trait/orderable.ts";
import { ObjectError } from "../error/error.ts";
import {
    AccessError,
    accessRelationship,
    isPrincipal,
    type Access,
    type Creation,
    type TupleReader,
    Manager,
} from "@destack/access";
import {
    TABLE,
    type InsertValue,
    type Select,
    type Expression,
    DatabaseError,
    Row,
    Condition,
    Scalar,
    type OpenCondition,
    Table,
} from "@destack/db";
import {
    nth,
    canonicalize,
    present,
    schema,
    zip,
    type JsonObject,
    type Version,
} from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { conceal, CallError } from "../error/call.ts";
import { MethodContext, RecordedRow } from "./context.ts";
import { Step } from "./step.ts";
import type * as sync from "@destack/sync";
import {
    defineMethod,
    Method,
    type MethodConfiguration,
    type MethodDefinition,
    type MethodSource,
} from "./method.ts";
import {
    Empty,
    PageFields,
    type MethodProcedure,
    type ObjectSchema,
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
import { EventLog } from "../query/event.ts";
import { Listing } from "../query/listing.ts";
import { ListFields, QueryFields, ViewFields } from "../replica/replica.ts";
import type { ObjectType, WrittenField } from "../object/object.ts";

import { versioned } from "../trait/versioned.ts";
import { TagMap } from "@destack/resource";

import { Execution, type StoreExecution } from "./execution.ts";

import type {} from "@destack/package/import-meta";

/** The most items one bulk creation takes, as many as one list page reads. */
export const BATCH_LIMIT = 1000;

/** The count of objects a bulk update or deletion changed. */
export const BatchPayload = schema.object({ count: schema.number().int().nonnegative() });

/** The identifiers of the objects a bulk creation created, in item order. */
export const BatchIds = schema.object({ ids: schema.array(schema.string()) });

/** One item of a bulk call: one creation's or one update's fields. */
export const BatchItem = schema.jsonObject();

/** The unique key a duplicate write broke, as the database reports it. */
const DUPLICATE_KEY = schema.looseObject({ columns: schema.array(schema.string()) });

/** The field values a bulk update matches objects by, by field name. */
const MATCH = schema.record(schema.string(), Scalar);

/** The object and revision an item of a bulk update refers to. */
const UPDATED_ITEM = schema.looseObject({
    id: schema.string(),
    revision: schema.number().int().positive(),
});

/** The column sets of a sum type, by discriminating field and then value. */
export type Variants<Fields extends string = string> = Readonly<
    Record<string, Readonly<Record<string, readonly Fields[]>>>
>;

/** How a creation method creates: its written fields, input, creator and prediction. */
export interface CreateOptions<
    Fields extends string,
    Input extends InputObject,
    Prepared extends schema.Schema,
    Internal extends boolean,
    Sum extends Variants = Variants,
    Filled extends string = string,
> {
    /** Whether the creation stays off public routes, setting the caller fields itself. */
    readonly isInternal?: Internal;
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
    Input extends InputObject,
    Prepared extends schema.Schema,
> {
    /** Whether the update stays off public routes. */
    readonly isInternal?: boolean;
    /** The fields the caller writes, every written field when absent. */
    readonly fields?: readonly Fields[];
    /** The caller-supplied values beyond fields. */
    readonly input?: Input;
    /** The external work the update prepares before its transaction. */
    readonly prepared?: Prepared;
    /** Whether clients predict the update. */
    readonly isPredicted?: boolean;
    /** The input fields each release computes from earlier input, by introducing release. */
    readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
}

/** How a bulk update matches and writes objects. */
export interface UpdateManyOptions<Fields extends string, Match extends string> {
    /** Whether the update stays off public routes. */
    readonly isInternal?: boolean;
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
    /** Whether the deletion stays off public routes. */
    readonly isInternal?: boolean;
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
>(
    permission: Permission,
    options: Omit<UpdateManyOptions<Fields, Match>, "match">,
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
}> {
    return {
        kind: "updateMany",
        source,
        permission,
        output: BatchPayload,
        mutates: true,
        ...(options.isInternal === undefined ? {} : { isInternal: options.isInternal }),
        ...(isItemized ? { isItemized } : {}),
        ...(options.convert === undefined ? {} : { convert: options.convert }),
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
            RecordMethod.requireUnrecorded(object);
            RecordMethod.requireUnguarded(object, options.fields);
        },
        handler: (context, execution) =>
            match === undefined
                ? RecordMethod.updateItems(context, execution, options.fields)
                : RecordMethod.updateMatching(context, execution, options.fields),
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
    Internal extends boolean = false,
    Target extends MethodTarget = "object",
> {
    /** The permission the caller needs on the target. */
    readonly permission: Permission;
    /** Whether the method stays off public routes, run only by the principals' code. */
    readonly isInternal?: Internal;
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
    /** The input fields each release computes from earlier input, by introducing release. */
    readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
    /** Whether clients predict the method once handled. */
    readonly isPredicted?: boolean;
}

/** The procedure each record method derives. */
export type RecordProcedures<Object extends ObjectType, Declared extends Method> = {
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
        IsSum<Declared["variants"]> extends true
            ? VariantSchema<Object, Declared, CreateFields<Object, Declared>>
            : schema.Object<CreateFields<Object, Declared>>,
        RowSchema<Object>
    >;
    createMany: Procedure<
        schema.Object<
            ScopeFields<Object> & ReplayFields<Object> & { items: ItemsSchema<Object, Declared> }
        >,
        schema.Object<{ ids: schema.Schema<string[]> }>
    >;
    update: Procedure<
        schema.Object<
            TargetFields<Object> &
                ReplayFields<Object> &
                typeof RevisionFields &
                WrittenFields<Object, Declared, true> &
                typeof TagFields &
                InputFields<Declared>
        >,
        RowSchema<Object>
    >;
    updateMany: Procedure<
        schema.Object<
            ScopeFields<Object> &
                ReplayFields<Object> &
                (NonNullable<Declared["isItemized"]> extends true
                    ? { items: UpdatedItemsSchema<Object, Declared> }
                    : {
                          where: schema.Object<MatchFields<Object, Declared>>;
                      } & WrittenFields<Object, Declared, true>)
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
                ReplayFields<Object> & { where: schema.Object<MatchFields<Object, Declared>> }
        >,
        schema.Object<{ count: schema.Schema<number> }>
    >;
    custom: Procedure<
        CustomSchema<
            (NonNullable<Declared["target"]> extends "scope"
                ? ScopeFields<Object>
                : TargetFields<Object>) &
                (Declared["mutates"] extends false ? {} : ReplayFields<Object>),
            Declared["input"]
        >,
        [NonNullable<Declared["output"]>] extends [never]
            ? RowSchema<Object>
            : NonNullable<Declared["output"]>
    >;
};

/** The fields of every creation: scope, chosen identifier, parent, replay, fields and input. */
type CreateFields<Object extends ObjectType, Declared extends Method> = ScopeFields<Object> &
    CreationFields<Object> &
    ParentFields<Object> &
    ReplayFields<Object> &
    WrittenFields<Object, Declared, false> &
    typeof TagFields &
    InputFields<Declared>;

/** The labels a creation or an update writes beside the fields, every record's. */
const TagFields = { tags: TagMap.exactOptional() };

/** The items of a bulk creation, each a creation's chosen identifier, parent and fields. */
type ItemsSchema<
    Object extends ObjectType,
    Declared extends Method,
    Item extends schema.Schema = schema.Object<
        CreationFields<Object> & ParentFields<Object> & WrittenFields<Object, Declared, false>
    >,
> = schema.Schema<schema.Output<Item>[], schema.Input<Item>[]>;

/** The items of a bulk update: each object's identifier and revision with its written fields. */
type UpdatedItemsSchema<
    Object extends ObjectType,
    Declared extends Method,
    Item extends schema.Schema = schema.Object<
        {
            id: schema.Schema<string, string>;
            revision: schema.Schema<number, number>;
        } & WrittenFields<Object, Declared, true>
    >,
> = schema.Schema<schema.Output<Item>[], schema.Input<Item>[]>;

/** Whether a method declares the column sets of a sum type. */
export type IsSum<Sum> = [NonNullable<Sum>] extends [never]
    ? false
    : object extends NonNullable<Sum>
      ? false
      : true;

/** The written values a call matches objects by, in their JSON form. */
type MatchFields<Object extends ObjectType, Declared extends Method> = ColumnFields<
    Object,
    string extends NonNullable<Declared["match"]>[number]
        ? WrittenField<Object["fields"]>
        : NonNullable<Declared["match"]>[number],
    true
>;

/** The identifier a caller may choose for a created object, and an event's identity and time. */
type CreationFields<Object extends ObjectType> = Object["storage"] extends "events"
    ? { id: IdSchema<Object>; time: schema.Schema<number, number> }
    : { id: schema.ExactOptional<IdSchema<Object>> };

/** The record methods' definitions and handlers: creation, update, deletion and their checks. */
export const RecordMethod = {
    /** Read a method's input fields. */
    fields(input: InputObject | undefined): InputObject["shape"] {
        return input?.shape ?? {};
    },

    /** Derive a creation's procedure at the collection, or by name below it for another. */
    createProcedure(
        name: string,
        schemas: ObjectSchema,
        options: Pick<
            CreateOptions<string, InputObject, schema.Schema, boolean>,
            "fields" | "isInternal" | "input" | "variants" | "filled"
        >,
    ): MethodProcedure {
        // take the shared fields, the caller leaving out those the handler fills
        const written = schemas.written(options.fields, false);
        const filled = Object.fromEntries(
            (options.filled ?? []).map((column) => [
                column,
                present(written[column], `the written field ${column}`).exactOptional(),
            ]),
        );
        const shared = schemas.scope.extend({
            ...schemas.created,
            ...schemas.parent,
            ...schemas.replay,
            ...written,
            ...TagFields,
            ...filled,
            ...(options.isInternal === true ? schemas.callers : {}),
            ...RecordMethod.fields(options.input),
        });

        return {
            route: { method: "POST", path: name === "create" ? "" : `/${Casing.kebab(name)}` },
            input:
                options.variants === undefined
                    ? shared
                    : RecordMethod.variantInput(name, schemas, shared, options.variants),
            output: schemas.row,
        };
    },

    /** Join each variant's columns to a creation's shared fields under its discriminating value. */
    variantInput(
        name: string,
        schemas: ObjectSchema,
        shared: InputObject,
        variants: Variants,
    ): InputSchema {
        // require one field telling the variants apart
        const [sum, ...others] = Object.entries(variants);
        if (sum === undefined || others.length > 0) {
            throw new TypeError(
                `creation ${name} needs exactly one field to tell its variants apart`,
            );
        }

        // take each variant's columns, required and non-null
        const [field, byValue] = sum;
        const objects = Object.entries(byValue).map(([value, written]) =>
            shared.extend({ [field]: schema.literal(value), ...schemas.required(written) }),
        );

        return InputSchema.union(field, objects);
    },

    /** Require the receiving parent and the create permission, over grants read once. */
    async authorizeCreate(
        context: MethodContext,
        execution: StoreExecution,
        grants?: TupleReader,
    ): Promise<void> {
        // require the receiving parent and the creation
        const reader = grants ?? (await execution.authorization.reader(context.scope));
        const parent = context.object.parent && context.parent();
        if (parent) {
            await RecordMethod.requireReceiving(context, execution, parent, reader);
        }
        await RecordMethod.requireCreatable(context, execution, reader);
    },

    /** Require the receive permission on a created object's parent. */
    async requireReceiving(
        context: MethodContext,
        execution: StoreExecution,
        parent: ObjectReference,
        reader?: TupleReader,
    ): Promise<void> {
        // require a nested object, and an attachment host type
        const declared = context.object.parent;
        if (declared === undefined) {
            throw new TypeError(`object ${context.object.name} is not nested in any parent`);
        }
        const { authorization } = execution;
        const authorizer = authorization.authorizer;
        if (declared.object === "any") {
            const hosts = authorizer.relation(context.object.policy, "parent").subjects;
            if (
                !hosts.some(
                    (host) => host.packageId === parent.packageId && host.type === parent.type,
                )
            ) {
                throw new ObjectError("PARENT_TYPE_REFUSED", {
                    message: `${parent.type} cannot have ${context.object.plural}`,
                });
            }

            // require the parent to exist
            const mapping = authorizer.requireMapping(parent);
            const [found] = await authorization.transaction
                .view()
                .read(mapping.table)
                .findMany({
                    where: {
                        [mapping.id]: parent.id,
                        ...(mapping.scope === undefined ? {} : { [mapping.scope]: parent.scope }),
                    },
                });
            if (found === undefined) {
                throw CallError.http("NOT_FOUND", {
                    message: `no ${parent.type} ${parent.id}`,
                });
            }
        }

        // hide an unreadable parent, else require the permission, unless the parent receives
        const receive = authorizer.policy(parent).permission(declared.receive);
        if (!(await authorization.check(receive, parent, reader)).isAllowed) {
            const host =
                declared.object === "any"
                    ? context.objects.find((object) => object.policy.is(parent))
                    : declared.object;
            const reading = host?.reading;
            if (reading === undefined || !(await authorization.check(reading, parent)).isAllowed) {
                throw CallError.http("NOT_FOUND", {
                    message: `no ${parent.type} ${parent.id}`,
                });
            }
            await authorization.require(receive, parent);
        }

        // require the permitted parent ready for children
        await RecordMethod.requireParentReady(context, parent.id);
    },

    /** Require each item's receiving parent and create permission, reading the grants once. */
    async authorizeItems(context: MethodContext, execution: StoreExecution): Promise<void> {
        const reader = await execution.authorization.reader(context.scope);
        await RecordMethod.eachItem(context, (item) =>
            RecordMethod.authorizeCreate(item, execution, reader),
        );
    },

    /** Insert a created object under an unused identifier and require guarded writes. */
    async insertAuthorized(
        context: MethodContext,
        execution: StoreExecution,
        insert: (created: MethodContext<MethodConfiguration, unknown, string>) => Promise<unknown>,
    ): Promise<{ readonly id: string; readonly row: unknown }> {
        const { id, created, row } = await RecordMethod.insertCreated(context, execution, insert);
        await execution.authorization.requireWritable(created, id, execution);

        return { id, row };
    },

    /** Create one object per item as a separate creation does, returning their identifiers. */
    async createItems(context: MethodContext, execution: Execution): Promise<{ ids: string[] }> {
        // append an events type's items to the event log in a store
        const { object } = context;
        if (object.storage === "events" && execution.kind === "store") {
            const rows = await RecordMethod.eachItem(context, async (item) =>
                RecordMethod.createdRow(
                    object.table,
                    await RecordMethod.createdValues(item, execution),
                ),
            );
            await EventLog.append(context, execution, rows);

            return { ids: rows.map((row) => schema.string().parse(row["id"])) };
        }

        // create independent items in one write in a store and require each to be writable
        const isSequential = object.ordering !== undefined || object.lifecycle.versioned;
        if (!isSequential && execution.kind === "store") {
            const ids = await RecordMethod.createTogether(context, execution);
            await RecordMethod.eachItem(context, (item, index) =>
                execution.authorization.requireWritable(item, nth(ids, index), execution),
            );

            return { ids };
        }

        // create one item at a time otherwise
        const ids = await RecordMethod.eachItem(context, async (item) => {
            // insert a prediction under the client's identifier
            if (execution.kind === "client") {
                const { id } = item;
                if (id === undefined) {
                    throw new TypeError(
                        `predicted ${item.object.name} creations need an identifier`,
                    );
                }
                await RecordMethod.createObject(item.identify(id), execution);

                return id;
            }
            // insert under an unused identifier and record its access
            else {
                const { id } = await RecordMethod.insertAuthorized(item, execution, (created) =>
                    RecordMethod.createObject(created, execution),
                );

                return id;
            }
        });

        return { ids };
    },

    /** Insert every item's row in one write, reporting the item a duplicate or creation refuses. */
    async createTogether(context: MethodContext, execution: StoreExecution): Promise<string[]> {
        // build each item's row under an unused identifier
        const created = await RecordMethod.eachItem(context, async (item) => {
            const id = await RecordMethod.createdId(item, execution);
            const row = RecordMethod.createdRow(
                context.object.table,
                await RecordMethod.createdValues(item.with({ id }), execution),
            );

            return { item: item.with({ id }), id, row };
        });

        // write the rows and record each creation's access within one savepoint
        const table = context.object.table;
        try {
            await execution.transaction.savepoint(async () => {
                await Execution.write(
                    execution,
                    context,
                    ...created.map(({ row }) => ({ table, before: null, after: row })),
                );
                for (const [index, { item, id }] of created.entries()) {
                    await RecordMethod.recordCreation(item, id, execution).catch(
                        (error: unknown) => {
                            throw RecordMethod.itemFailure(context.object, index, error);
                        },
                    );
                }
            });
        } catch (error) {
            // report the first item duplicating a stored row or an earlier item, as the caller sees it
            const index =
                error instanceof DatabaseError && error.reason === "DUPLICATE"
                    ? await RecordMethod.duplicateIndex(context, execution, created, error)
                    : undefined;
            const failing = index === undefined ? undefined : created[index];
            if (index === undefined || failing === undefined) {
                throw error;
            }
            await RecordMethod.requireCreatable(failing.item, execution);
            throw RecordMethod.itemFailure(context.object, index, error);
        }

        return created.map(({ id }) => id);
    },

    /** Find the first created row repeating a stored or an earlier row's duplicate key. */
    async duplicateIndex(
        context: MethodContext,
        execution: StoreExecution,
        created: readonly { readonly row: Row }[],
        error: DatabaseError,
    ): Promise<number | undefined> {
        // read each row's value of the duplicate key's columns, by field
        const definition = context.object.table[TABLE];
        const columns = DUPLICATE_KEY.parse(error.key).columns;
        const keyFields = columns.map(
            (column) =>
                present(
                    Object.entries(definition.columns).find(
                        ([, declared]) => declared.definition.name === column,
                    ),
                    `the field of ${context.object.name}'s column ${column}`,
                )[0],
        );
        const keys = created.map(({ row }) =>
            Object.fromEntries(keyFields.map((field) => [field, row[field]])),
        );

        // pass a key of structured values, which no equality condition reads
        const matched = keys.map((key) => MATCH.safeParse(key));
        if (matched.some((entry) => !entry.success)) {
            return undefined;
        }

        // find the first row repeating a stored or an earlier key
        const conditions = matched.flatMap((entry) =>
            entry.success ? [Condition.equal(entry.data)] : [],
        );
        const stored = await execution.transaction
            .read(context.object.table)
            .findMany({ where: { OR: conditions } });
        const taken = new Set(
            stored.map((found) => {
                const row: Row = found;

                return canonicalize(keyFields.map((field) => row[field]));
            }),
        );
        const index = keys.findIndex((key) => {
            // take a key seen before, and remember this one
            const value = canonicalize(keyFields.map((field) => key[field]));
            const isTaken = taken.has(value);
            taken.add(value);

            return isTaken;
        });

        return index < 0 ? undefined : index;
    },

    /** Run a call per item of a bulk creation in order, prefixing a failure with its index. */
    async eachItem<Result>(
        context: MethodContext,
        run: (item: MethodContext, index: number) => Promise<Result>,
    ): Promise<Result[]> {
        const results: Result[] = [];
        for (const [index, { id, ...written }] of BatchItem.array()
            .parse(context.input["items"])
            .entries()) {
            // run the item as a separate creation's call
            const item = context.with({
                input: written,
                ...(id === undefined ? {} : { id: schema.string().parse(id) }),
            });
            try {
                results.push(await run(item, index));
            } catch (error) {
                throw RecordMethod.itemFailure(context.object, index, error);
            }
        }

        return results;
    },

    /** Prefix a failing item's call error with its index, mapping a conflict to its fields. */
    itemFailure(object: ObjectType, index: number, error: unknown): unknown {
        // pass other failures as they are
        const conflict = object.conflict(error);
        const failure = CallError.of(conflict);
        if (failure === undefined) {
            return conflict;
        }

        return new CallError(failure.code, {
            reason: failure.reason,
            domain: failure.domain,
            status: failure.status,
            message: `items.${index}: ${failure.message}`,
            data: failure.data,
            cause: failure,
        });
    },

    /** Delete each object a bulk creation created, undoable only by a type with a delete method. */
    deleteCreated(step: Step): readonly sync.Call[] | undefined {
        const { ids } = BatchIds.parse(step.result);
        const calls = ids.flatMap(
            (id) => Step.call(step, "delete", Step.reference(step, id)) ?? [],
        );

        return calls.length === 0 ? undefined : calls;
    },

    /** Restore the updated fields a step wrote that stayed unchanged since. */
    restoreUpdated(step: Step): readonly sync.Call[] | undefined {
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
    },

    /** Refuse tracked and versioned objects, whose change records bulk calls skip. */
    requireUnrecorded(object: ObjectType): void {
        if (object.lifecycle.tracked !== undefined || object.lifecycle.versioned) {
            throw new TypeError(
                `object ${object.name} records each change, so it cannot apply many changes at once`,
            );
        }
    },

    /** Refuse guarded fields, whose write permission bulk updates skip. */
    requireUnguarded(object: ObjectType, written: readonly string[]): void {
        const guarded = written.find((name) => object.fields[name]?.access?.write !== undefined);
        if (guarded !== undefined) {
            throw new TypeError(
                `object ${object.name} guards field ${guarded}, so it cannot apply many updates at once`,
            );
        }
    },

    /** Update every matching object the caller may change in one write, returning the count. */
    async updateMatching(
        context: MethodContext,
        execution: Execution,
        written: readonly string[],
    ): Promise<{ count: number }> {
        // stamp the written fields over each match, refusing a managed object
        const table = context.object.table;
        const values = RecordMethod.writtenValues(table, context.input, written);
        const rows = await RecordMethod.matchingRows(context, execution);
        const changes = rows.map((row) => {
            const each = context.at(row);
            RecordMethod.requireUnmanaged(each, execution);

            return {
                table,
                before: row,
                after: Table.values(table, Row.parse({ ...row, ...each.revised(values) })),
            };
        });

        // write every match at once
        await Execution.write(execution, context, ...changes);

        return { count: changes.length };
    },

    /** Update each listed object at its revision in one write, all or none. */
    async updateItems(
        context: MethodContext,
        execution: Execution,
        written: readonly string[],
    ): Promise<{ count: number }> {
        // read the listed objects in the call's scope, filtering the permitted ones on the server
        const table = context.object.table;
        const items = BatchItem.array().parse(context.input["items"]);
        const named = items.map((item) => UPDATED_ITEM.parse(item));
        const listed: OpenCondition = {
            scope: context.scope,
            id: { in: named.map((item) => item.id) },
        };
        const rows =
            execution.kind === "client"
                ? await context.read(table).findMany({ where: listed })
                : (
                      await execution.authorization.filter(
                          context.object,
                          context.permission(),
                          await execution.transaction.read(table).findMany({ where: listed }),
                      )
                  ).rows;
        const targets = new Map(rows.map((row) => [RecordedRow.parse(row).id, row]));

        // stamp each item's written fields at its revision, as an update of one object does
        const changes = zip(items, named).map(([item, { id, revision }]) => {
            // refuse a missing or unpermitted object
            const target = targets.get(id);
            if (target === undefined) {
                throw CallError.http("NOT_FOUND", {
                    message: `${context.object.name} not found`,
                });
            }
            // refuse a stale revision
            else if (RecordedRow.parse(target).revision !== revision) {
                throw new ObjectError("STALE_REVISION", {
                    message: `${context.object.name} revision has changed`,
                });
            }

            // write the item's fields over its target, refusing a managed object
            const each = context.at(target);
            RecordMethod.requireUnmanaged(each, execution);
            const values = RecordMethod.writtenValues(table, item, written);

            return {
                table,
                before: target,
                after: Table.values(table, Row.parse({ ...target, ...each.revised(values) })),
            };
        });

        // write every item at once
        await Execution.write(execution, context, ...changes);

        return { count: changes.length };
    },

    /** Decode the written fields an input gives values. */
    writtenValues(table: Table, input: JsonObject, written: readonly string[]): Row {
        return Table.decode(
            table,
            Object.fromEntries(
                written.flatMap((name) => {
                    const value = input[name];

                    return value === undefined ? [] : [[name, value]];
                }),
            ),
        );
    },

    /** Delete every matching object the caller may delete, each as its deletion does. */
    async deleteMatching(context: MethodContext, execution: Execution): Promise<{ count: number }> {
        // take each match as its target
        const rows = await RecordMethod.matchingRows(context, execution);
        const targets = rows.map((row) => context.at(row));
        for (const target of targets) {
            RecordMethod.requireUnmanaged(target, execution);
        }

        // delete each match on a device as a separate deletion
        if (execution.kind === "client") {
            for (const target of targets) {
                await RecordMethod.deleteObject(target);
            }
        }
        // delete or trash the read matches in one write
        else {
            const table = context.object.table;
            await Execution.write(
                execution,
                context,
                ...targets.map((target) => ({ table, ...RecordMethod.deletion(target) })),
            );
        }

        return { count: rows.length };
    },

    /** Read a loaded target's deletion: its trashed row under a trash, else its removal. */
    deletion(target: MethodContext<{ readonly table: Table }, unknown>): {
        readonly before: Row;
        readonly after: Row | null;
    } {
        // remove an object deleting at once
        const before: Row = target.target;
        if (!target.object.isTrashable()) {
            return { before, after: null };
        }

        // trash it at the next revision, recording who asked
        const trashed = target.revised(RecordMethod.trashed(target));

        return {
            before,
            after: Table.values(target.object.table, Row.parse({ ...before, ...trashed })),
        };
    },

    /** Read the columns trashing a call's target: the time, and the subject asking. */
    trashed(context: MethodContext) {
        return {
            deletedAt: context.now,
            ...ObjectReference.columns("deletedBy", context.caller.subject),
        };
    },

    /** Read the untrashed objects a bulk call matches, the permitted ones on a server. */
    async matchingRows(context: MethodContext, execution: Execution) {
        // match the values in the call's scope, leaving out objects whose deletion is requested
        const table = context.object.table;
        const matched = MATCH.parse(context.input["where"]);
        const isDeleting = Object.hasOwn(table[TABLE].columns, "deletedAt");
        const where = Condition.equal({
            ...matched,
            ...(isDeleting ? { deletedAt: null } : {}),
            scope: context.scope,
        });

        // read the matches, filtering the permitted ones on the server
        const rows =
            execution.kind === "client"
                ? await context.read(table).findMany({ where })
                : (
                      await execution.authorization.filter(
                          context.object,
                          context.permission(),
                          await execution.transaction.read(table).findMany({ where }),
                      )
                  ).rows;

        // refuse matching a predefined row
        rows.forEach((row) => context.object.refusePredefined(RecordedRow.parse(row).id));

        return rows;
    },

    /** Insert a created object. */
    async createObject(
        context: MethodContext<MethodConfiguration, unknown, string>,
        execution: Execution,
    ): Promise<Select<Table>> {
        // write the created row, then record its access
        const row = RecordMethod.createdRow(
            context.object.table,
            await RecordMethod.createdValues(context, execution),
        );
        await Execution.write(execution, context, {
            table: context.object.table,
            before: null,
            after: row,
        });
        await RecordMethod.recordCreation(context, context.id, execution);

        return row;
    },

    /** Record a created object's access: a new scope's chain and its first relationships. */
    async recordCreation(context: MethodContext, id: string, execution: Execution): Promise<void> {
        const creating = RecordMethod.readCreation(context);
        const isScope = context.object.policy.definition.scope === true;
        if (execution.kind === "store" && (isScope || creating !== undefined)) {
            const object = context.object.reference(context.scope, id);
            await execution.authorization.create(object, (await creating?.(context, object)) ?? {});
        }
    },

    /** Build the columns a created object is inserted with. */
    async createdValues(context: MethodContext, execution: Execution): Promise<InsertValue<Table>> {
        // fill caller fields with a calling principal of the declared kinds, a reference by its identifier column
        const { object } = context;
        const caller = context.caller.subject;
        const isInternal = context.method.isInternal === true;
        const callers: Record<string, string | null> = {};
        for (const [name, declared] of Object.entries(object.fields)) {
            const kinds = declared.principals;
            if (!declared.isCallerFilled || isInternal || declared.reference === "none") {
                continue;
            } else if (
                caller === undefined ||
                !(kinds === undefined ? isPrincipal(caller) : kinds.some((kind) => kind.is(caller)))
            ) {
                const named = kinds?.map((kind) => kind.name).join(" or ") ?? "principal";
                throw new ObjectError("CREATOR_KIND_REFUSED", {
                    message: `only a ${named} can create ${object.name}`,
                });
            }
            Object.assign(
                callers,
                declared.reference === "object"
                    ? ObjectReference.columns(declared.relationAt(name), caller)
                    : { [name]: caller.id },
            );
        }

        // number versions within their parent
        const version = object.lifecycle.versioned
            ? await versioned.next(object, context.requireParent().id, context)
            : undefined;

        // place an ordered object after its last sibling unless the caller placed it
        const decoded = Table.decode(object.table, context.input);
        const parent = object.parent === undefined ? {} : context.parentColumns();
        const position =
            object.ordering === undefined || context.input["position"] !== undefined
                ? undefined
                : await Orderable.next(
                      object,
                      { ...decoded, ...parent, scope: context.scope },
                      context,
                  );

        // require the writer
        if (object.storage === "ephemeral" && context.writerId === undefined) {
            throw new TypeError(`only a writer can create ephemeral ${object.name} objects`);
        }

        // assemble the columns, stamping the times and the author where the table records them
        const columns = object.table[TABLE].columns;
        const isTimed = Object.hasOwn(columns, "createdAt");
        const isAuthored = Object.hasOwn(columns, "createdById");
        const manager = execution.kind === "store" ? execution.authorization.manager : null;
        const managed = manager === null || !object.isDeclarable() ? {} : Manager.values(manager);

        return {
            ...decoded,
            ...parent,
            ...(position === undefined ? {} : { position }),
            id: context.id,
            scope: context.scope,
            ...(version === undefined ? {} : { number: version }),
            ...(object.storage === "ephemeral" ? { writerId: context.writerId } : {}),
            ...callers,
            ...(isTimed ? { createdAt: context.now, updatedAt: context.now } : {}),
            ...(isAuthored
                ? {
                      ...ObjectReference.columns("createdBy", caller),
                      ...ObjectReference.columns("updatedBy", caller),
                  }
                : {}),
            ...managed,
        };
    },

    /** Insert a created object under an unused identifier within a savepoint. */
    async insertCreated(
        context: MethodContext,
        execution: StoreExecution,
        insert: (created: MethodContext<MethodConfiguration, unknown, string>) => Promise<unknown>,
    ): Promise<{
        readonly id: string;
        readonly created: MethodContext<MethodConfiguration, unknown, string>;
        readonly row: unknown;
    }> {
        // insert within a savepoint
        try {
            return await execution.transaction.savepoint(async () => {
                // insert under an unused identifier
                const id = await RecordMethod.createdId(context, execution);
                const created = context.identify(id);

                return { id, created, row: await insert(created) };
            });
        } catch (error) {
            // report conflicts only to authorized callers
            const isConflict =
                CallError.of(error)?.code === "CONFLICT" ||
                (error instanceof DatabaseError &&
                    (error.reason === "DUPLICATE" || error.reason === "BROKEN_REFERENCE"));
            if (isConflict) {
                await RecordMethod.requireCreatable(context, execution);
            }
            throw error;
        }
    },

    /** Build a created object's complete row from its written values and the columns' defaults. */
    createdRow(table: Table, values: InsertValue<Table>): Row {
        return Table.complete(table, Table.values(table, Row.parse(values)));
    },

    /** Require the create permission on the object as written. */
    async requireCreatable(
        context: MethodContext,
        execution: StoreExecution,
        reader?: TupleReader,
    ): Promise<void> {
        // build the row as it will be written, with column defaults
        const id = context.id ?? context.object.generateId();
        const definition = context.object.table[TABLE];
        const values = await RecordMethod.createdValues(context.with({ id }), execution);
        const row = definition.defaults(Table.values(context.object.table, Row.parse(values)));

        // grant the creation's first relationships, then check the permission on the row
        const { authorization } = execution;
        const grants = reader ?? (await authorization.reader(context.scope));
        const created = context.object.reference(context.scope, id);
        const initial =
            (await RecordMethod.readCreation(context)?.(context.with({ id }), created)) ?? {};
        const { authorizer } = authorization;
        grants.creating(created, authorizer.initialRelationships(created, initial, context.now));
        await RecordMethod.requirePermitted(context, execution, row, grants, "concealed");
    },

    /** Require a scoped method's permission in the call's scope, as on a new object there. */
    async authorizeScoped(
        context: MethodContext,
        execution: StoreExecution,
        selected: ((input: JsonObject) => Row) | undefined,
    ): Promise<void> {
        // read the selected columns, else the input's, and a nested type's parent
        const definition = context.object.table[TABLE];
        const input: JsonObject = context.input;
        const named =
            selected?.(input) ??
            Object.fromEntries(
                Object.keys(definition.columns)
                    .filter((column) => input[column] !== undefined)
                    .map((column) => [column, input[column]]),
            );
        const parent = context.parentColumns();

        // check the permission on a new object with those columns, under an unused key for a keyed type
        const id =
            context.object.keyed === undefined ? context.object.generateId() : crypto.randomUUID();
        const row = definition.defaults({ ...named, id, scope: context.scope, ...parent });
        await RecordMethod.requirePermitted(
            context,
            execution,
            row,
            await execution.authorization.reader(context.scope),
            "open",
        );
    },

    /** Require the call's permission on a row, challenging for step-up authentication. */
    async requirePermitted(
        context: MethodContext,
        execution: StoreExecution,
        row: Row,
        grants: TupleReader,
        refusal: "open" | "concealed",
    ): Promise<void> {
        // check the permission on the row
        const { authorization } = execution;
        const { authorizer, snapshot } = authorization;
        const permission = context.permission();
        const allows = async (access: Access) =>
            (await authorizer.checkRows(snapshot, permission, access, [row], grants)).permitted.has(
                0,
            );
        const access = await authorization.in(context.scope);
        if (await allows(access)) {
            return;
        }

        // challenge for step-up authentication
        const stepUp = await authorizer.challenge(snapshot, permission, access, allows);
        if (stepUp !== undefined) {
            throw new AccessError(
                "INSUFFICIENT_AUTHENTICATION",
                "authentication assurance is too low: authenticate again",
                { stepUp },
            );
        }

        // refuse openly, or hide an object the caller cannot read
        const reading = context.object.reading;
        const isReadable =
            refusal === "open" ||
            (reading !== undefined &&
                (await authorizer.checkRows(snapshot, reading, access, [row])).permitted.has(0));

        // refuse a reader, and hide the object from others
        const denial = new ObjectError("PERMISSION_DENIED", {
            message: `permission denied: ${permission.name}`,
        });
        throw isReadable ? denial : conceal(denial, `${context.object.name} not found`);
    },

    /** Delete an object, or request deletion when a trash or controller finishes it. */
    async deleteObject(
        context: MethodContext<{ readonly table: Table }, unknown>,
    ): Promise<Record<string, never>> {
        // refuse deleting an object twice
        const { object } = context;
        const target: Row = context.target;
        if (target["deletedAt"] !== undefined && target["deletedAt"] !== null) {
            throw new ObjectError("ALREADY_DELETED", {
                message: `${object.name} is already deleted`,
            });
        }

        // request deletion when a trash or controller finishes it, moving the children nested to the trash with it
        if (object.isTrashable()) {
            const trashed = RecordMethod.trashed(context);
            await context.update(trashed);
            await RecordMethod.trashChildren(
                context,
                schema.string().parse(target["id"]),
                "delete",
            );

            return {};
        }

        // delete at the loaded revision
        await context.remove();

        return {};
    },

    /** Trash a deleted object's live children with it, or restore the ones it trashed. */
    async trashChildren(
        context: MethodContext<{ readonly table: Table }, unknown>,
        parentId: string,
        verb: "delete" | "restore",
    ): Promise<void> {
        // find the types moving to the trash with this one
        const types = context.objects.filter(
            (type) =>
                type.parent?.delete === "trash" &&
                type.parent.object !== "any" &&
                type.parent.object.same(context.object),
        );
        for (const type of types) {
            // read its live children, or those the parent's deletion moved to the trash
            const where =
                verb === "delete"
                    ? { parentId, deletedAt: { isNull: true } }
                    : { parentId, isTrashedWithParent: true };
            const children = await context.read<Table>(type.table).findMany({ where });
            if (children.length === 0) {
                continue;
            }

            // run each child's verb as the parent's type, which the child's refusals still guard
            const calls = children.map((child) =>
                type.call(
                    verb,
                    type.target({ scope: context.scope, id: RecordedRow.parse(child).id }),
                ),
            );
            await context.mutate(context.self, calls);

            // mark them trashed with the parent, or no longer
            const written = await context.read<Table>(type.table).findMany({
                where: { id: { in: children.map((child) => RecordedRow.parse(child).id) } },
            });
            for (const child of written) {
                await context
                    .with({ object: type })
                    .at(child)
                    .update({ isTrashedWithParent: verb === "delete" });
            }
        }
    },

    /** Refuse a child under a missing parent or one in the trash. */
    async requireParentReady(context: MethodContext, parentId: string | null): Promise<void> {
        // check a child of a parent type with a trash
        const declared = context.object.parent;
        if (
            declared === undefined ||
            declared.object === "any" ||
            !declared.object.isTrashable() ||
            parentId === null
        ) {
            return;
        }
        const [parent] = await context
            .read<Table>(declared.object.table)
            .findMany({ where: { id: parentId } });

        // refuse a missing parent and one in the trash
        if (parent === undefined) {
            throw CallError.http("NOT_FOUND", {
                message: `no ${declared.object.name} ${parentId}`,
            });
        } else if (parent["deletedAt"] !== null) {
            throw new ObjectError("IN_TRASH", {
                message: `${declared.object.name} ${parentId} is in the trash`,
            });
        }
    },

    /** Accept a caller's unused identifier, or generate one, random in a prediction. */
    async createdId(context: MethodContext, execution: StoreExecution): Promise<string> {
        // generate a missing identifier
        const { object, id: chosen } = context;
        if (chosen === undefined) {
            return context.generateId(object);
        }

        // find a row with the identifier, or a relationship a deleted object left under it
        const { packageId, name } = object.policy.definition;
        const id = object.idSchema.parse(chosen);
        const rows = await execution.transaction
            .view()
            .read<Table>(object.table)
            .findMany({ where: { id } });
        const relationships = await execution.authorization.transaction
            .view()
            .read(accessRelationship)
            .findMany({
                where: {
                    objectPackageId: packageId,
                    objectType: name,
                    objectId: id,
                },
            });
        const isTaken = rows.length > 0 || relationships.length > 0;

        // refuse a taken identifier, as a lost race when a later commit took it
        if (isTaken) {
            await execution.transaction.requireCurrent();
            await execution.authorization.transaction.requireCurrent();
            throw new ObjectError("ID_TAKEN", {
                message: `${object.name} identifier is taken`,
            });
        }

        return chosen;
    },

    /** Refuse changes to a managed object from anyone but its manager. */
    requireUnmanaged(context: MethodContext, execution: Execution): void {
        // let the record's manager through
        const manager = execution.kind === "store" ? execution.authorization.manager : null;
        if (context.isDeclarable() && context.target !== undefined) {
            Manager.requireChangeable(context.target, manager, context.object.name);
        }
    },

    /** Read how a call creates its object's first relationships. */
    readCreation(
        context: MethodContext,
    ):
        | ((context: MethodContext, object: ObjectReference) => Creation | Promise<Creation>)
        | undefined {
        const hasOwnerRelation =
            context.object.roles !== undefined && context.object.policy.definition.scope === true;

        return (
            context.method.creation ??
            (hasOwnerRelation ? RecordMethod.relateCreator("owner") : undefined)
        );
    },

    /** Relate the caller to a new object, and a new scope's owner role to its holders. */
    relateCreator(relation: string): (context: MethodContext, object: ObjectReference) => Creation {
        return (call, object) => {
            // relate the creator
            const caller = call.caller.subject;
            const isScope = call.object.policy.definition.scope === true;

            return {
                relationships: [{ relation, subject: caller }],
                ...(isScope ? { owner: { ...object, relation } } : {}),
            };
        };
    },
};

/** The method declarations of one object type, typed by its table, fields and permissions. */
export class MethodBuilder<
    Definition extends Table = Table,
    Written extends string = string,
    Granted extends string = string,
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
            handler: (context, execution) => new Listing(context, execution).list(),
        });
    }

    /** Create an object from the fields the caller writes. */
    create<
        const Permission extends Granted,
        const Fields extends Written = Written,
        Input extends InputObject = never,
        Prepared extends schema.Schema = never,
        const Internal extends boolean = false,
        const Sum extends Variants = never,
        const Filled extends Fields = never,
    >(
        this: MethodBuilder<Definition>,
        permission: Permission,
        options: CreateOptions<Fields, Input, Prepared, Internal, Sum, Filled> = {},
    ) {
        // relate the creation's creator
        const { creator, ...declared } = options;

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
            internal: NoInfer<Internal>;
            target: "scope";
        }>({
            kind: "create",
            source: { builder: "create", arguments: [permission, options] },
            permission,
            mutates: true,
            ...declared,
            target: "scope",
            result: "object",
            procedure: (name, schemas) => RecordMethod.createProcedure(name, schemas, options),
            ...(creator === undefined ? {} : { creation: RecordMethod.relateCreator(creator) }),
            handler: (context, execution) => RecordMethod.createObject(context, execution),
            authorize: (context, execution) => RecordMethod.authorizeCreate(context, execution),
            inverse: (step) => {
                // delete the created object
                const call = Step.call(step, "delete", Step.target(step));

                return call === undefined ? undefined : [call];
            },
            async execute(context, execution) {
                // predict under the client's identifier
                if (execution.kind === "client") {
                    if (context.id === undefined) {
                        throw new TypeError(
                            `predicted ${context.object.name} creations need an identifier`,
                        );
                    }

                    return Method.handle(this, context, execution);
                }

                // append an events type's object to the event log
                if (context.object.storage === "events") {
                    const row = RecordMethod.createdRow(
                        context.object.table,
                        await RecordMethod.createdValues(context, execution),
                    );
                    await EventLog.append(context, execution, [row]);

                    return row;
                }

                // insert under an unused identifier and record its access
                const { row } = await RecordMethod.insertAuthorized(
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
        const Fields extends Written = Written,
        const Internal extends boolean = false,
    >(permission: Permission, options: CreateManyOptions<Fields, Internal>) {
        // relate the creation's creator
        const { creator, ...declared } = options;

        return defineMethod<{
            kind: "createMany";
            permission: Permission;
            output: typeof BatchIds;
            mutates: true;
            fields: NoInfer<Fields>;
            table: Definition;
            internal: NoInfer<Internal>;
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
                                ...(options.isInternal === true ? schemas.callers : {}),
                            }),
                        )
                        .max(BATCH_LIMIT),
                }),
                output: BatchIds,
            }),
            validate: (object) => RecordMethod.requireUnrecorded(object),
            ...(creator === undefined ? {} : { creation: RecordMethod.relateCreator(creator) }),
            handler: (context, execution) => RecordMethod.createItems(context, execution),
            authorize: (context, execution) => RecordMethod.authorizeItems(context, execution),
            inverse: (step) => RecordMethod.deleteCreated(step),
        });
    }

    /** Update an object's written fields. */
    update<
        const Permission extends Granted,
        const Fields extends Written = Written,
        Input extends InputObject = never,
        Prepared extends schema.Schema = never,
    >(permission: Permission, options: UpdateOptions<Fields, Input, Prepared> = {}) {
        return defineMethod<{
            kind: "update";
            target: "object";
            permission: Permission;
            input: NoInfer<Input>;
            prepared: NoInfer<Prepared>;
            mutates: true;
            fields: NoInfer<Fields>;
            table: Definition;
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
                    ...RecordMethod.fields(options.input),
                }),
                output: schemas.row,
            }),
            handler: (context) => context.update(Table.decode(context.object.table, context.input)),
            inverse: (step) => RecordMethod.restoreUpdated(step),
            async execute(context, execution) {
                // refuse managed objects and unwritable fields
                RecordMethod.requireUnmanaged(context, execution);
                if (execution.kind === "store") {
                    await execution.authorization.requireWritable(context, context.id, execution);
                }

                return Method.handle(this, context, execution);
            },
        });
    }

    /** Update every matching object the caller may change, returning the count. */
    updateMany<
        const Permission extends Granted,
        const Fields extends Written = Written,
        const Match extends string = string,
    >(permission: Permission, options: UpdateManyOptions<Fields, Match>) {
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
        }>(
            bulkUpdate(permission, options, options.match, false, {
                builder: "updateMany",
                arguments: [permission, options],
            }),
        );
    }

    /** Update each listed object at its revision, all or none, returning the count. */
    updateItems<const Permission extends Granted, const Fields extends Written = Written>(
        permission: Permission,
        options: Omit<UpdateManyOptions<Fields, never>, "match">,
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
        }>(
            bulkUpdate(permission, options, undefined, true, {
                builder: "updateItems",
                arguments: [permission, options],
            }),
        );
    }
    /** Delete an object. */
    delete<const Permission extends Granted, Prepared extends schema.Schema = never>(
        permission: Permission,
        options: RemoveOptions<Prepared> = {},
    ) {
        return defineMethod<{
            kind: "delete";
            target: "object";
            permission: Permission;
            output: typeof Empty;
            prepared: NoInfer<Prepared>;
            mutates: true;
            table: Definition;
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
            handler: (context) => RecordMethod.deleteObject(context),
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
                RecordMethod.requireUnmanaged(context, execution);

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
            validate: (object) => RecordMethod.requireUnrecorded(object),
            handler: (context, execution) => RecordMethod.deleteMatching(context, execution),
        });
    }

    /** Declare a method reading one object, as tRPC's `query`. */
    query<
        const Permission extends Granted,
        Input extends InputSchema = never,
        Output extends schema.Schema = never,
        Prepared extends schema.Schema = never,
        const Internal extends boolean = false,
        const Target extends MethodTarget = "object",
    >(definition: CustomDefinition<Permission, Input, Output, Prepared, Internal, Target>) {
        return defineMethod<{
            kind: "custom";
            permission: Permission;
            input: NoInfer<Input>;
            output: NoInfer<Output>;
            prepared: NoInfer<Prepared>;
            mutates: false;
            table: Definition;
            internal: NoInfer<Internal>;
            target: NoInfer<Target>;
        }>(this.#custom(definition, false));
    }

    /** Declare a method changing one object, as tRPC's `mutation`. */
    mutation<
        const Permission extends Granted,
        Input extends InputSchema = never,
        Output extends schema.Schema = never,
        Prepared extends schema.Schema = never,
        const Internal extends boolean = false,
        const Target extends MethodTarget = "object",
    >(definition: CustomDefinition<Permission, Input, Output, Prepared, Internal, Target>) {
        return defineMethod<{
            kind: "custom";
            permission: Permission;
            input: NoInfer<Input>;
            output: NoInfer<Output>;
            prepared: NoInfer<Prepared>;
            mutates: true;
            table: Definition;
            internal: NoInfer<Internal>;
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
        const Internal extends boolean,
        const Target extends MethodTarget,
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
        table: Definition;
        internal: Internal;
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
                          RecordMethod.authorizeScoped(context, execution, selected),
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

import {
    createInsertSchema,
    createSelectSchema,
    type Insert,
    type Select,
    TABLE,
} from "@destack/db";
import { Scope } from "@destack/sync";
import { identifier, schema, type Identifier } from "@destack/schema";
import { defineProcedure } from "@destack/service/procedure";
import { RequestId } from "@destack/service/request";
import { ParentReference } from "../trait/nested.ts";
import type { ObjectType } from "../object/object.ts";
import { kebabCase } from "../object/name.ts";
import { ClientId } from "../replica/replica.ts";
import type { Method } from "./method.ts";
import type { RecordProcedures } from "../trait/record.ts";
import type { RecoverableProcedures } from "../trait/recoverable.ts";
import type { NestedProcedures } from "../trait/nested.ts";
import type { ShareableProcedures } from "../trait/shareable.ts";
import type { DetachableProcedures } from "../trait/declarable.ts";
import type { TrackedProcedures } from "../trait/tracked.ts";
import type { TextProcedures } from "../trait/text.ts";
import type { TransitionProcedures } from "../trait/transition.ts";

/** The schemas an object's procedures compose. */
export interface ObjectSchema {
    /** The object as callers see it. */
    readonly row: schema.Object<Record<string, schema.Schema>>;
    /** The fields selecting the scope. */
    readonly scope: schema.Object<Record<string, schema.Schema>>;
    /** The fields selecting one object. */
    readonly target: schema.Object<Record<string, schema.Schema>>;
    /** The identifier a caller may choose for a created object. */
    readonly created: Record<string, schema.Schema>;
    /** The parent a created object belongs to. */
    readonly parent: Record<string, schema.Schema>;
    /** The parent a moved object moves to. */
    readonly destination: Record<string, schema.Schema>;
    /** The field naming a mutation: its request identifier or client. */
    readonly replay: Record<string, schema.Schema>;
    /** Read the columns a write takes. */
    written(
        names: readonly string[] | undefined,
        isPartial: boolean,
    ): Record<string, schema.Schema>;
}

/** An HTTP method and a path below an object's collection. */
export interface Route {
    /** The HTTP method. */
    readonly method: "GET" | "POST" | "PATCH" | "DELETE";
    /** The path below the collection, empty for the collection itself. */
    readonly path: "" | `/${string}`;
}

/** How callers reach one method. */
export interface MethodProcedure {
    /** The route below the object's collection. */
    readonly route: Route;
    /** The complete input. */
    readonly input: schema.Schema;
    /** The output: a value or a stream of values. */
    readonly output: ProcedureOutput;
}

/** The fields paging a list. */
export const PageShape = {
    /** The continuation from the previous page. */
    cursor: schema.string().min(1).optional(),
    /** The largest page to return. */
    limit: schema.number().int().min(1).max(1000).optional(),
};

/** The revision a change requires. */
export const RevisionShape = { revision: schema.number().int().positive().optional() };

/** The empty result of methods returning nothing. */
export const Empty = schema.object({});

/** Derive one procedure per method of an object type. */
export function objectProcedures<Object extends ObjectType>(
    object: Object,
): ObjectProcedures<Object> {
    // build the shapes
    const scope = scopeRoute(object);
    const collection = `${scope.prefix}/${kebabCase(object.plural)}`;
    const shapes = objectSchema(object);

    // derive each method's procedure
    const procedures: Record<string, unknown> = {};
    for (const [name, method] of Object.entries(object.methods) as [string, Method][]) {
        if (method.isSystem) {
            continue;
        }
        const { route, input, output } = method.procedure(name, shapes);
        procedures[name] = procedure(
            defineProcedure({
                authentication: method.mutates ? "identity" : "public",
                permission: null,
                audit: false,
            }),
            { method: route.method, path: `${collection}${route.path}` as `/${string}` },
            input,
            output,
        );
    }

    return procedures as ObjectProcedures<Object>;
}

/** Build the schemas an object's procedures compose. */
export function objectSchema(object: ObjectType): ObjectSchema {
    // read the identifier and row schemas
    const idColumn = object.table[TABLE].columns.id;
    if (idColumn === undefined) {
        throw new TypeError(`object ${object.name} is held in a table without an id column`);
    }
    const id = idColumn.definition.schema;
    const columns = createSelectSchema(object.table, "json");
    const selected = schema.object(
        Object.fromEntries(
            Object.entries(columns.shape as Record<string, schema.Schema>).filter(
                ([name]) => !object.sensitive.includes(name),
            ),
        ),
    );
    const parent = object.parent && parentSchema(object.parent.object);
    const field = scopeRoute(object).field;
    const selection = schema.object(field === undefined ? {} : { [field]: schema.string().min(1) });

    // make guarded fields optional and read text fields as strings
    const row = selected.extend({
        ...Object.fromEntries(
            object.guarded.map((name) => [name, selected.shape[name]!.optional()]),
        ),
        ...Object.fromEntries(object.text.map((name) => [name, schema.string()])),
    });

    // read the written columns' JSON schemas
    const inserted = createInsertSchema(object.table, "json").shape as Record<
        string,
        schema.Schema
    >;

    return {
        row,
        written: (names, isPartial) =>
            Object.fromEntries(
                (names ?? object.written).map((name) => {
                    // take each written column's schema
                    const declared = inserted[name];
                    if (declared === undefined) {
                        throw new TypeError(`${object.name} writes no column ${name}`);
                    }

                    return [name, isPartial ? declared.optional() : declared];
                }),
            ),
        scope: selection,
        target: selection.extend({ id }),
        created: { id: id.optional() },
        parent: Object.fromEntries(
            Object.entries(parent ?? {}).map(([name, column]) => [
                name,
                object.parent!.optional ? column.optional() : column,
            ]),
        ),
        destination: Object.fromEntries(
            Object.entries(parent ?? {}).map(([name, column]) => [
                name,
                object.parent!.optional ? column.nullable() : column,
            ]),
        ),
        replay:
            object.storage === "ephemeral" ? { client: ClientId } : { requestId: RequestId.schema },
    };
}

/** Build the schema naming an object's parent. */
function parentSchema(parent: ObjectType | "any"): Record<string, schema.Schema> {
    return parent === "any"
        ? { parent: ParentReference }
        : { parentId: identifier(parent.identity) };
}

/** The route prefix and input field selecting an object's scope. */
export interface ScopeRoute {
    /** The path prefix, such as /spaces/{spaceId}. */
    readonly prefix: "" | `/${string}`;
    /** The input field holding the scope identifier. */
    readonly field?: string;
}

/** Derive an object's scope route. */
export function scopeRoute(object: ObjectType): ScopeRoute {
    // route global objects without a scope
    const scope = object.scope;
    if (scope === Scope.universe.id) {
        return { prefix: "" };
    }
    // name the scope in a field for objects in several
    else if (Array.isArray(scope)) {
        return { prefix: "", field: "scope" };
    }

    // name the scope in its type's route and identifier field
    const single = scope as ObjectType;

    return {
        prefix: `/${kebabCase(single.plural)}/{${single.identity}Id}`,
        field: `${single.identity}Id`,
    };
}

/** The schema a procedure outputs: a value or a stream of values. */
export type ProcedureOutput = Parameters<ReturnType<typeof defineProcedure>["output"]>[0];

/** Declare one derived procedure with its access, route, input and output. */
function procedure<Input extends schema.Schema, Output extends ProcedureOutput>(
    access: ReturnType<typeof defineProcedure>,
    route: { readonly method: Route["method"]; readonly path: `/${string}` },
    input: Input,
    output: Output,
) {
    return access.route(route).input(input).output(output);
}

/** A derived procedure with its input and output schemas. */
export type Procedure<Input extends schema.Schema, Output extends ProcedureOutput> = ReturnType<
    typeof procedure<Input, Output>
>;

/** The procedures derived from an object type's methods. */
export type ObjectProcedures<Object extends ObjectType> = {
    readonly [Name in CallableName<Object>]: MethodProcedureOf<
        Object,
        Object["methods"][Name] & Method
    >;
};

/** The names of an object type's non-system methods. */
export type CallableName<Object extends ObjectType> = {
    [Name in keyof Object["methods"] & string]: Object["methods"][Name] extends {
        readonly isSystem: true;
    }
        ? never
        : Name;
}[keyof Object["methods"] & string];

/** The procedure one method derives, by its kind. */
type MethodProcedureOf<Object extends ObjectType, Declared extends Method> = (RecordProcedures<
    Object,
    Declared
> &
    RecoverableProcedures<Object> &
    TransitionProcedures<Object> &
    NestedProcedures<Object> &
    ShareableProcedures<Object> &
    DetachableProcedures<Object> &
    TrackedProcedures<Object> &
    TextProcedures<Object>)[Declared["kind"]];

/** The input field of an object's scope identifier. */
export type ScopeField<Object extends ObjectType> = Object["scope"] extends "universe"
    ? never
    : Object["scope"] extends ObjectType
      ? `${ScopeIdentity<Object["scope"]>}Id`
      : "scope";

/** The prefix of a scope type's identifiers. */
export type ScopeIdentity<Scope extends ObjectType> =
    Select<Scope["table"]>["id" & keyof Select<Scope["table"]>] extends Identifier<infer Prefix>
        ? Prefix
        : never;

/** The fields selecting an object's scope. */
export type ScopeShape<Object extends ObjectType> = {
    [Field in ScopeField<Object>]: schema.Schema<string, string>;
};

/** The fields selecting one object. */
export type TargetShape<Object extends ObjectType> = ScopeShape<Object> & {
    id: schema.Schema<Select<Object["table"]>["id" & keyof Select<Object["table"]>]>;
};

/** The input field naming the parent. */
type ParentField<Object extends ObjectType> = "parentType" extends keyof Select<Object["table"]>
    ? "parent"
    : "parentId";

/** The value naming an object's parent. */
type ParentValue<Object extends ObjectType> = "parentType" extends keyof Select<Object["table"]>
    ? ParentReference
    : NonNullable<Select<Object["table"]>["parentId" & keyof Select<Object["table"]>]>;

/** Whether an object may have no parent. */
type IsOrphanable<Object extends ObjectType> = null extends Select<Object["table"]>["parentId" &
    keyof Select<Object["table"]>]
    ? true
    : false;

/** A child's parent-selecting field. */
export type ParentShape<
    Object extends ObjectType,
    Filter extends boolean,
> = "parentId" extends keyof Select<Object["table"]>
    ? {
          [Field in ParentField<Object>]: Filter extends true
              ? schema.Optional<schema.Schema<ParentValue<Object>>>
              : IsOrphanable<Object> extends true
                ? schema.Optional<schema.Schema<ParentValue<Object>>>
                : schema.Schema<ParentValue<Object>>;
      }
    : {};

/** The parent a moved object moves to. */
export type DestinationShape<Object extends ObjectType> = "parentId" extends keyof Select<
    Object["table"]
>
    ? {
          [Field in ParentField<Object>]: IsOrphanable<Object> extends true
              ? schema.Schema<ParentValue<Object> | null>
              : schema.Schema<ParentValue<Object>>;
      }
    : {};

/** The field naming a mutation: its request identifier or client. */
export type ReplayShape<Object extends ObjectType = ObjectType> =
    Object["storage"] extends "ephemeral"
        ? { client: typeof ClientId }
        : { requestId: typeof RequestId.schema };

/** The revision a change requires. */
export type RevisionField = { revision: schema.Optional<schema.Schema<number, number>> };

/** The keys a record may omit. */
type OptionalKeys<Record> = {
    [Key in keyof Record]-?: {} extends Pick<Record, Key> ? Key : never;
}[keyof Record];

/** The columns a creation or update writes. */
type WrittenName<Object extends ObjectType, Declared extends Method> = string extends NonNullable<
    Declared["fields"]
>[number]
    ? Object["written"][number]
    : NonNullable<Declared["fields"]>[number];

/** A value in its JSON form. */
type JsonForm<Value> = Value extends Date
    ? number
    : Value extends bigint
      ? string
      : Value extends Uint8Array
        ? string
        : Value;

/** The JSON validators of the columns a creation or update writes. */
export type WrittenShape<
    Object extends ObjectType,
    Declared extends Method,
    IsPartial extends boolean,
    Row = Insert<Object["table"]>,
    Name extends keyof Row = Extract<WrittenName<Object, Declared>, keyof Row>,
> = {
    [
        Key in Name as IsPartial extends true ? never : Key extends OptionalKeys<Row> ? never : Key
    ]: schema.Schema<JsonForm<Row[Key]>>;
} & {
    [
        Key in Name as IsPartial extends true ? Key : Key extends OptionalKeys<Row> ? Key : never
    ]: schema.Optional<schema.Schema<JsonForm<Exclude<Row[Key], undefined>>>>;
};

/** A method's own input fields. */
export type FieldShape<Declared extends Method> = [NonNullable<Declared["input"]>] extends [never]
    ? {}
    : NonNullable<Declared["input"]> extends schema.Object<infer Shape>
      ? Shape
      : {};

/** One object as callers see it, in its JSON form. */
export type RowSchema<Object extends ObjectType> = schema.Object<
    RowShape<
        JsonRow<Omit<Select<Object["table"]>, Object["sensitive"][number]>>,
        Object["guarded"][number]
    > & { [Name in Object["text"][number]]: schema.Schema<string> }
>;

/** A row with each field in its JSON form. */
type JsonRow<Row> = { [Name in keyof Row]: JsonForm<Row[Name]> };

/** The validators of a row's fields, the guarded ones accepting omission. */
type RowShape<Row, Guarded> = {
    [Name in Exclude<keyof Row, Guarded>]: schema.Schema<Row[Name]>;
} & {
    [Name in Extract<keyof Row, Guarded>]: schema.Optional<schema.Schema<Row[Name]>>;
};

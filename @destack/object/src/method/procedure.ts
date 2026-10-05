import { type Insert, type JsonOf, type Select, TABLE } from "@destack/db";
import type * as sync from "@destack/sync";
import { schema } from "@destack/schema";
import { defineProcedure } from "@destack/service/procedure";
import { RequestId } from "@destack/service/request";
import { ParentReference } from "../trait/nested.ts";
import type {
    CallerField,
    GuardedField,
    ObjectScope,
    ObjectType,
    SensitiveField,
    TextFieldName,
    WrittenField,
} from "../object/object.ts";
import type { MethodKind } from "./kind.ts";
import { kebabCase } from "../object/name.ts";
import { ClientId } from "../replica/replica.ts";
import type { Method } from "./method.ts";
import type { RecordProcedures } from "./record.ts";
import type { RecoverableProcedures } from "../trait/recoverable.ts";
import type { NestedProcedures } from "../trait/nested.ts";
import type { ShareableProcedures } from "../trait/shareable.ts";
import type { DeclarableProcedures, DetachableProcedures } from "../trait/declarable.ts";
import type { BindableProcedures } from "../trait/bindable.ts";
import type { TrackedProcedures } from "../trait/tracked.ts";
import type { TextProcedures } from "../trait/text.ts";
import type { TransitionProcedures } from "../trait/transition.ts";

/** The schemas an object's procedures compose. */
export interface ObjectSchema {
    /** The object as callers see it. */
    readonly row: schema.Object<Record<string, schema.Schema>>;
    /** The fields selecting the scope. */
    readonly scope: schema.JsonObject;
    /** The fields selecting one object. */
    readonly target: schema.JsonObject;
    /** The identifier a caller may choose for a created object. */
    readonly created: schema.JsonObject["shape"];
    /** The parent a created object belongs to. */
    readonly parent: schema.JsonObject["shape"];
    /** The parent a moved object moves to. */
    readonly destination: schema.JsonObject["shape"];
    /** The field naming a mutation: its request identifier or client. */
    readonly replay: schema.JsonObject["shape"];
    /** The columns a system creation writes for the calling principal it names. */
    readonly callers: schema.JsonObject["shape"];
    /** Read the columns a write takes. */
    written(names: readonly string[] | undefined, isPartial: boolean): schema.JsonObject["shape"];
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
    /** The complete input, an object of fields. */
    readonly input: schema.JsonObject;
    /** The output: a value or a stream of values. */
    readonly output: ProcedureOutput;
}

/** The fields paging a list. */
export const PageShape = {
    /** The continuation from the previous page. */
    cursor: schema.string().min(1).exactOptional(),
    /** The largest page to return. */
    limit: schema.number().int().min(1).max(1000).exactOptional(),
};

/** The revision a change requires. */
export const RevisionShape = { revision: schema.number().int().positive().exactOptional() };

/** The empty result of methods returning nothing. */
export const Empty = schema.object({});

/** Derive one procedure per method of an object type. */
export function objectProcedures<Object extends ObjectType>(
    object: Object,
): ObjectProcedures<Object>;
/**
 * Derive one procedure per method of an object type.
 *
 * @construct each method gets the procedure of its own kind, input and output, which is how ObjectProcedures maps the methods.
 */
export function objectProcedures(object: ObjectType): Readonly<Record<string, unknown>> {
    return callableProcedures(object);
}

/** Derive one procedure per callable method of an object type, each parsing its input to JSON. */
export function callableProcedures(
    object: ObjectType,
): Readonly<Record<string, Procedure<schema.JsonObject, ProcedureOutput>>> {
    // build the shapes
    const scope = scopeRoute(object);
    const collection: `/${string}` = `${scope.prefix}/${kebabCase(object.plural)}`;
    const shapes = objectSchema(object);

    // derive each method's procedure
    const procedures: Record<string, Procedure<schema.JsonObject, ProcedureOutput>> = {};
    for (const [name, method] of Object.entries(object.methods)) {
        if (method.isSystem === true) {
            continue;
        }
        const { route, input, output } = method.procedure(name, shapes);
        procedures[name] = procedure(
            defineProcedure({
                authentication: method.mutates ? "identity" : "public",
                permission: null,
                audit: false,
                convert: object.conversions(name),
            }),
            { method: route.method, path: `${collection}${route.path}` },
            input,
            output,
        );
    }

    return procedures;
}

/** Build the schemas an object's procedures compose. */
export function objectSchema(object: ObjectType): ObjectSchema {
    // read the identifier and the fields selecting the scope
    const id = object.table[TABLE].column("id").definition.json;
    const scopeField = scopeRoute(object).field;
    const selection = schema.object(
        scopeField === undefined ? {} : { [scopeField]: schema.string().min(1) },
    );

    // read the written columns, and the caller columns a system creation writes
    const written = writtenSchema(object);
    const callers = Object.entries(object.fields)
        .filter(([, declared]) => declared.isCallerFilled)
        .map(([name]) => name);

    return {
        row: rowSchema(object),
        written,
        callers: written(callers, false),
        scope: selection,
        target: selection.extend({ id }),
        created: { id: id.exactOptional() },
        ...parentShapes(object),
        replay:
            object.storage === "ephemeral" ? { client: ClientId } : { requestId: RequestId.schema },
    };
}

/** Build an object's row as callers see it: guarded fields optional and text fields as strings. */
function rowSchema(object: ObjectType): ObjectSchema["row"] {
    // leave out the sensitive columns
    const columns = object.table[TABLE].selectSchema("json");
    const readable = Object.entries(columns.shape).filter(
        ([name]) => !object.sensitive.includes(name),
    );

    // make guarded fields optional and read text fields as strings
    return schema.object({
        ...Object.fromEntries(
            readable.map(([name, column]) => [
                name,
                object.guarded.includes(name) ? column.exactOptional() : column,
            ]),
        ),
        ...Object.fromEntries(object.text.map((name) => [name, schema.string()])),
    });
}

/** Build the reader of the written columns' schemas. */
function writtenSchema(object: ObjectType): ObjectSchema["written"] {
    // read the written columns' JSON schemas
    const inserted = object.table[TABLE].insertSchema("json").shape;

    return (names, isPartial) =>
        Object.fromEntries(
            (names ?? object.written).map((name) => {
                // take each written column's schema
                const field = object.fields[name];
                const value = object.table[TABLE].columns[name]?.definition.json;
                const isRequired = field?.required === true && field.access?.read !== undefined;

                // require a guarded field its nullable column would leave optional
                const required = field?.initial === undefined ? value : value?.exactOptional();
                const declared = isRequired ? required : inserted[name];
                if (declared === undefined) {
                    throw new TypeError(`${object.name} writes no column ${name}`);
                }

                return [name, isPartial ? declared.exactOptional() : declared];
            }),
        );
}

/** Build the parent a created object belongs to and the parent a moved object moves to. */
function parentShapes(object: ObjectType): Pick<ObjectSchema, "parent" | "destination"> {
    // read the parent columns of a nested object
    const nesting = object.parent;
    const parent = nesting && parentSchema(nesting.object);

    return {
        parent: Object.fromEntries(
            Object.entries(parent ?? {}).map(([name, column]) => [
                name,
                nesting?.optional === true ? column.exactOptional() : column,
            ]),
        ),
        destination: Object.fromEntries(
            Object.entries(parent ?? {}).map(([name, column]) => [
                name,
                nesting?.optional === true ? column.nullable() : column,
            ]),
        ),
    };
}

/** Build the schema naming an object's parent. */
function parentSchema(parent: ObjectType | "any"): schema.JsonObject["shape"] {
    return parent === "any" ? { parent: ParentReference } : { parentId: parent.idSchema };
}

/** The route prefix and input field selecting an object's scope. */
export interface ScopeRoute {
    /** The path prefix, such as /spaces/{spaceId}. */
    readonly prefix: "" | `/${string}`;
    /** The input field with the scope identifier. */
    readonly field?: string;
}

/** Derive an object's scope route. */
export function scopeRoute(object: ObjectType): ScopeRoute {
    // name the scope in a field for objects in several, and route global objects without one
    const declared: readonly ObjectScope[] = [object.scope].flat();
    const [single] = object.scopes;
    if (declared.length > 1) {
        return { prefix: "", field: "scope" };
    } else if (single === undefined) {
        return { prefix: "" };
    }

    // name the scope in its type's route and identifier field

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

/** The procedures an object type's methods derive, by method name. */
export type MethodProcedures<Object extends ObjectType> = {
    readonly [Name in MethodName<Object>]: MethodProcedureOf<
        Object,
        Object["methods"][Name] & Method
    >;
};

/** The procedures callers call: those of an object type's non-system methods, by method name. */
export type ObjectProcedures<Object extends ObjectType> = Pick<
    MethodProcedures<Object>,
    CallableName<Object>
>;

/** The names of an object type's methods. */
export type MethodName<Object extends ObjectType> = keyof Object["methods"] & string;

/** The names of an object type's non-system methods. */
export type CallableName<Object extends ObjectType> = {
    [Name in keyof Object["methods"] & string]: [
        NonNullable<Object["methods"][Name]["isSystem"]>,
    ] extends [true]
        ? never
        : Name;
}[keyof Object["methods"] & string];

/** The procedure of a method whose kind is unknown: any input and output. */
type AnyProcedure = Procedure<
    schema.Object<Readonly<Record<string, schema.Schema>>>,
    ProcedureOutput
>;

/** The procedure one method derives, by its kind, any procedure for a method of unknown kind. */
type MethodProcedureOf<
    Object extends ObjectType,
    Declared extends Method,
> = MethodKind extends Declared["kind"]
    ? AnyProcedure
    : (RecordProcedures<Object, Declared> &
          RecoverableProcedures<Object> &
          TransitionProcedures<Object> &
          NestedProcedures<Object> &
          ShareableProcedures<Object> &
          DetachableProcedures<Object> &
          DeclarableProcedures<Object> &
          BindableProcedures &
          TrackedProcedures<Object> &
          TextProcedures<Object>)[Declared["kind"]];

/** The names of an object type's mutating methods. */
export type MutationName<Object extends ObjectType> = {
    [Name in CallableName<Object>]: Object["methods"][Name] extends { mutates: true }
        ? Name
        : never;
}[CallableName<Object>];

/** The calls of an object type's mutating methods, recorded to run later. */
export type Calls<Object extends ObjectType> = {
    readonly [Name in MutationName<Object>]: (input: CallInput<Object, Name>) => sync.Call;
};

/** The procedure one method derives. */
type ProcedureOf<
    Object extends ObjectType,
    Name extends MethodName<Object>,
> = MethodProcedures<Object>[Name] & {
    readonly "~orpc": { readonly inputSchema: schema.Schema; readonly outputSchema: schema.Schema };
};

/** The input a caller passes to a method, without the scope it calls in. */
export type CallInput<Object extends ObjectType, Name extends MethodName<Object>> =
    string extends MethodName<Object>
        ? Readonly<Record<string, unknown>>
        : Omit<
              schema.Input<ProcedureOf<Object, Name>["~orpc"]["inputSchema"]>,
              | "requestId"
              | (Object["storage"] extends "ephemeral" ? "client" : never)
              | ScopeField<Object>
          >;

/** The result a method returns. */
export type CallOutput<Object extends ObjectType, Name extends MethodName<Object>> = schema.Infer<
    ProcedureOf<Object, Name>["~orpc"]["outputSchema"]
>;

/** The input field of an object's scope identifier. */
export type ScopeField<Object extends ObjectType> =
    Object extends ObjectType<infer Configuration>
        ? Configuration["scope"] extends "universe"
            ? never
            : string extends Configuration["scope"]
              ? "scope"
              : `${Configuration["scope"]}Id`
        : never;

/** The fields selecting an object's scope. */
export type ScopeShape<Object extends ObjectType> = {
    [Field in ScopeField<Object>]: schema.Schema<string, string>;
};

/** The fields selecting one object. */
export type TargetShape<Object extends ObjectType> = ScopeShape<Object> & {
    id: schema.Schema<JsonOf<Select<Object["table"]>["id" & keyof Select<Object["table"]>]>>;
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
export type ParentShape<Object extends ObjectType> = "parentId" extends keyof Select<
    Object["table"]
>
    ? {
          [Field in ParentField<Object>]: IsOrphanable<Object> extends true
              ? schema.ExactOptional<schema.Schema<ParentValue<Object>>>
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
export type RevisionField = typeof RevisionShape;

/** The keys a record may omit. */
type OptionalKeys<Record> = {
    [Key in keyof Record]-?: {} extends Pick<Record, Key> ? Key : never;
}[keyof Record];

/** The columns a creation or update writes. */
type WrittenName<Object extends ObjectType, Declared extends Method> =
    | (string extends NonNullable<Declared["fields"]>[number]
          ? WrittenField<Object["fields"]>
          : NonNullable<Declared["fields"]>[number])
    | (Declared extends { readonly kind: "create" }
          ? true extends NonNullable<Declared["isSystem"]>
              ? CallerField<Object["fields"]>
              : never
          : never);

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
    ]: schema.Schema<JsonOf<Row[Key]>, JsonOf<Row[Key]>>;
} & {
    [
        Key in Name as IsPartial extends true ? Key : Key extends OptionalKeys<Row> ? Key : never
    ]: schema.ExactOptional<
        schema.Schema<JsonOf<Exclude<Row[Key], undefined>>, JsonOf<Exclude<Row[Key], undefined>>>
    >;
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
        JsonFields<Omit<Select<Object["table"]>, SensitiveField<Object["fields"]>>>,
        GuardedField<Object["fields"]>
    > & { [Name in TextFieldName<Object["fields"]>]: schema.Schema<string> }
>;

/** Fields, each in its JSON form. */
type JsonFields<Row> = { [Name in keyof Row]: JsonOf<Row[Name]> };

/** The validators of a row's fields, the guarded ones accepting omission. */
type RowShape<Row, Guarded> = {
    [Name in Exclude<keyof Row, Guarded>]: schema.Schema<Row[Name]>;
} & {
    [Name in Extract<keyof Row, Guarded>]: schema.ExactOptional<schema.Schema<Row[Name]>>;
};

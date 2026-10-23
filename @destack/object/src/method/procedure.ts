import { type Insert, type JsonOf, jsonShape, type Select, TABLE } from "@destack/db";
import { Scope, RequestId } from "@destack/sync";
import { type Identifier, mapValues, schema } from "@destack/schema";
import { Package } from "@destack/package";
import { defineProcedure } from "../procedure/index.ts";
import { MAX_PAGE_LIMIT } from "./page.ts";
import { ParentReference } from "../trait/nested.ts";
import type {
    CallerField,
    GuardedField,
    ObjectType,
    SensitiveField,
    TextFieldName,
    UpdatedField,
    WrittenField,
} from "../object/object.ts";
import type { MethodKind } from "./kind.ts";
import { Casing } from "../object/name.ts";
import { WriterId } from "../replica/replica.ts";
import type { Method } from "./method.ts";
import type { InputObject, InputSchema } from "./input.ts";
import type { RecordProcedures } from "./record.ts";
import type { RecoverableProcedures } from "../trait/recoverable.ts";
import type { NestedProcedures } from "../trait/nested.ts";
import type { ShareableProcedures } from "../trait/shareable.ts";
import type { BindableProcedures } from "../trait/bindable.ts";
import type { TrackedProcedures } from "../trait/tracked.ts";
import type { TextProcedures } from "../trait/text.ts";
import type { TransitionProcedures } from "../trait/transition.ts";

/** The schemas an object's procedures compose. */
export interface ObjectSchema {
    /** The object as callers see it. */
    readonly row: schema.Object<Record<string, schema.Schema>>;
    /** The fields selecting the scope. */
    readonly scope: InputObject;
    /** The fields selecting one object. */
    readonly target: InputObject;
    /** The identifier a creation may choose, and an event's identity and time. */
    readonly created: InputObject["shape"];
    /** The parent a created object belongs to. */
    readonly parent: InputObject["shape"];
    /** The parent a moved object moves to. */
    readonly destination: InputObject["shape"];
    /** The field identifying a mutation: its request identifier or client. */
    readonly replay: InputObject["shape"];
    /** The columns an internal creation writes for the calling principal it records. */
    readonly callers: InputObject["shape"];
    /** Read the columns a write takes. */
    readonly written: (
        names: readonly string[] | undefined,
        isPartial: boolean,
    ) => InputObject["shape"];
    /** Read the columns a write must set to non-null, as one sum type variant writes them. */
    readonly required: (names: readonly string[]) => InputObject["shape"];
    /** Read the columns a transition records, an optional one cleared by null. */
    readonly recorded: (names: readonly string[]) => InputObject["shape"];
}

/** An HTTP method and a path below an object's collection. */
export interface Route {
    /** The HTTP method. */
    readonly method: "GET" | "POST" | "PATCH" | "DELETE";
    /** The path below the collection, empty for the collection itself. */
    readonly path: "" | `/${string}`;
}

/** How callers call one method. */
export interface MethodProcedure {
    /** The route below the object's collection. */
    readonly route: Route;
    /** The complete input: an object of fields, or one object per variant of a sum type. */
    readonly input: InputSchema;
    /** The output: a value or a stream of values. */
    readonly output: ProcedureOutput;
}

/** The fields paging a list. */
export const PageFields = {
    /** The continuation from the previous page. */
    cursor: schema.string().min(1).exactOptional(),
    /** The largest page to return. */
    limit: schema.number().int().min(1).max(MAX_PAGE_LIMIT).exactOptional(),
};

/** A list method's page: its rows and the cursor of the next page, null after the last. */
export interface ListPage<Item> {
    /** The page's rows. */
    readonly items: readonly Item[];
    /** The cursor continuing after the page, null after the last page. */
    readonly cursor: string | null;
}

/** The pages of list methods. */
export const ListPage = {
    /** Describe a page of some rows. */
    schema<Item extends schema.Schema>(item: Item) {
        return schema.object({
            /** The page's rows. */
            items: schema.array(item),
            /** The cursor continuing after the page, null after the last page. */
            cursor: schema.string().min(1).nullable(),
        });
    },

    /** Read every row of a list, following its cursors page by page. */
    async *all<Item, Input extends { readonly cursor?: string }>(
        list: (input: Input) => Promise<ListPage<Item>>,
        input: Input,
    ): AsyncGenerator<Item> {
        let cursor: string | null | undefined;
        do {
            // read the next page after the cursor
            const page = await list(
                cursor === undefined || cursor === null ? input : { ...input, cursor },
            );
            yield* page.items;
            cursor = page.cursor;
        } while (cursor !== null);
    },
};

/** The revision a change requires. */
export const RevisionFields = { revision: schema.number().int().positive().exactOptional() };

/** The empty result of methods returning nothing. */
export const Empty = schema.object({});

/** Derive one procedure per method of an object type. */
export function objectProcedures<Object extends ObjectType>(
    object: Object,
): ObjectProcedures<Object> {
    return mapValues<
        Readonly<Record<string, Procedure<InputSchema, ProcedureOutput>>>,
        ObjectProcedures<Object>
    >(callableProcedures(object), (procedure) => procedure);
}

/** Derive one procedure per callable method of an object type, each parsing its input to JSON. */
export function callableProcedures(
    object: ObjectType,
): Readonly<Record<string, Procedure<InputSchema, ProcedureOutput>>> {
    // derive each method's procedure below its collection, in its scope's route
    const procedures: Record<string, Procedure<InputSchema, ProcedureOutput>> = {};
    for (const [name, method] of Object.entries(object.methods)) {
        if (method.isInternal === true) {
            continue;
        }
        const collection: `/${string}` = `${object.route.prefix}/${Casing.kebab(object.plural)}`;
        const { route, input, output } = object.procedure(name);
        procedures[name] = ObjectProcedure.procedure(
            defineProcedure({
                package: Package.of(object.package),
                authentication: method.mutates ? "identity" : "public",
                permission: { rows: object.policy.permission(method.permission) },
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
    const scopeField = object.route.field;
    const selection = schema.object(
        scopeField === undefined ? {} : { [scopeField]: schema.string().min(1) },
    );

    // read the written columns, and the caller columns an internal creation writes
    const written = ObjectProcedure.writtenSchema(object);
    const callers = Object.entries(object.fields)
        .filter(([, declared]) => declared.isCallerFilled)
        .map(([name]) => name);

    return {
        row: ObjectProcedure.rowSchema(object),
        written,
        required: (names) =>
            Object.fromEntries(
                names.map((name) => [name, object.table[TABLE].column(name).definition.json]),
            ),
        recorded: (names) =>
            Object.fromEntries(
                names.map((name) => {
                    // take an optional field's null, which clears it
                    const { definition } = object.table[TABLE].column(name);
                    const value = definition.nullable
                        ? schema.nullable(definition.json)
                        : definition.json;

                    return [name, value];
                }),
            ),
        callers: written(callers, false),
        scope: selection,
        target: selection.extend({ id }),
        created:
            object.storage === "events"
                ? { id, time: object.table[TABLE].column("time").definition.json }
                : { id: id.exactOptional() },
        ...ObjectProcedure.parentShapes(object),
        replay:
            object.storage === "ephemeral"
                ? { writerId: WriterId }
                : { requestId: RequestId.schema },
    };
}

/** The route prefix and input field selecting an object's scope. */
export interface ScopeRoute {
    /** The path prefix, such as /spaces/{spaceId}. */
    readonly prefix: "" | `/${string}`;
    /** The input field with the scope identifier. */
    readonly field?: string;
}

/** Derive the scope route of an object type, or of one of its methods. */
export function scopeRoute(object: ObjectType): ScopeRoute {
    // route universe objects without a scope, and put the scope in a field for several or any
    const [single, ...others] = object.scopes;
    const scopes: readonly unknown[] = [object.scope].flat();
    if (object.scope === Scope.universe.id) {
        return { prefix: "" };
    } else if (single === undefined || others.length > 0 || scopes.includes(Scope.universe.id)) {
        return { prefix: "", field: "scope" };
    }

    // put the scope in its type's route and identifier field
    return {
        prefix: `/${Casing.kebab(single.plural)}/{${single.identity}Id}`,
        field: `${single.identity}Id`,
    };
}

/** The schema a procedure outputs: a value or a stream of values. */
export type ProcedureOutput = Parameters<ReturnType<typeof defineProcedure>["output"]>[0];

/** A derived procedure with its input and output schemas. */
export type Procedure<Input extends schema.Schema, Output extends ProcedureOutput> = ReturnType<
    typeof ObjectProcedure.procedure<Input, Output>
>;

/** The procedures an object type's methods derive, by method name. */
export type MethodProcedures<Object extends ObjectType> = {
    readonly [Name in MethodName<Object>]: MethodProcedureOf<
        Object,
        Object["methods"][Name] & Method
    >;
};

/** The procedures callers call: those of an object type's routed methods, by method name. */
export type ObjectProcedures<Object extends ObjectType> = Pick<
    MethodProcedures<Object>,
    CallableName<Object>
>;

/** The names of an object type's methods. */
export type MethodName<Object extends ObjectType> = keyof Object["methods"] & string;

/** The names of an object type's routed methods. */
export type CallableName<Object extends ObjectType> = {
    [Name in keyof Object["methods"] & string]: [
        NonNullable<Object["methods"][Name]["isInternal"]>,
    ] extends [true]
        ? never
        : Name;
}[keyof Object["methods"] & string];

/** The procedure of a method whose kind is unknown: any input and output. */
type AnyProcedure = Procedure<
    | schema.Object<Readonly<Record<string, schema.Schema>>>
    | schema.DiscriminatedUnion<readonly schema.Object<Readonly<Record<string, schema.Schema>>>[]>,
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
          TransitionProcedures<Object, Declared> &
          NestedProcedures<Object> &
          ShareableProcedures<Object> &
          BindableProcedures &
          TrackedProcedures<Object> &
          TextProcedures<Object>)[Declared["kind"]];

/** The names of an object type's mutating methods. */
export type MutationName<Object extends ObjectType> = {
    [Name in CallableName<Object>]: Object["methods"][Name] extends { mutates: true }
        ? Name
        : never;
}[CallableName<Object>];

/** The procedure one method derives. */
type ProcedureOf<
    Object extends ObjectType,
    Name extends MethodName<Object>,
> = MethodProcedures<Object>[Name] & {
    readonly "~orpc": { readonly inputSchema: schema.Schema; readonly outputSchema: schema.Schema };
};

/** A method's input as a client gives it, without scope and request identifiers, per variant. */
export type MethodInput<Object extends ObjectType, Name extends MethodName<Object>> =
    string extends MethodName<Object>
        ? Readonly<Record<string, unknown>>
        : OmitEach<
              schema.Input<ProcedureOf<Object, Name>["~orpc"]["inputSchema"]>,
              | "requestId"
              | (Object["storage"] extends "ephemeral" ? "writerId" : never)
              | ScopeField<Object>
          >;

/** A method's input as its handler reads it, without scope and request identifiers, per variant. */
export type ParsedInput<Object extends ObjectType, Name extends MethodName<Object>> =
    string extends MethodName<Object>
        ? Readonly<Record<string, unknown>>
        : OmitEach<
              schema.Infer<ProcedureOf<Object, Name>["~orpc"]["inputSchema"]>,
              | "requestId"
              | (Object["storage"] extends "ephemeral" ? "writerId" : never)
              | ScopeField<Object>
          >;

/** A call's input: the method's input with its scope, the running context's if absent. */
export type CallInput<Object extends ObjectType, Name extends MethodName<Object>> =
    string extends MethodName<Object>
        ? Readonly<Record<string, unknown>>
        : MethodInput<Object, Name> & CallScope<Object>;

/** The scope of a call, the running call's when absent; a store requires it. */
export type CallScope<Object extends ObjectType> = {
    readonly [Field in ScopeField<Object>]?: string;
};

/** The input placing a call in a scope: the type's scope field. */
export type ScopeInput<Object extends ObjectType> = {
    readonly [Field in ScopeField<Object>]: string;
};

/** The input identifying one object as a call's target: its scope field and its identifier. */
export type TargetInput<Object extends ObjectType> = ScopeInput<Object> & {
    readonly id: string;
};

/** Leave keys out of each member of a union. */
export type OmitEach<Value, Key extends PropertyKey> = Value extends unknown
    ? Omit<Value, Key>
    : never;

/** The result a method returns. */
export type MethodOutput<Object extends ObjectType, Name extends MethodName<Object>> = schema.Infer<
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
export type ScopeFields<Object extends ObjectType> = {
    [Field in ScopeField<Object>]: schema.Schema<string, string>;
};

/** The fields selecting one object. */
export type TargetFields<Object extends ObjectType> = ScopeFields<Object> & {
    id: IdSchema<Object>;
};

/** The validator of an object's identifier, accepting the plain string its brand reads. */
export type IdSchema<Object extends ObjectType> = schema.Schema<
    JsonOf<Select<Object["table"]>["id" & keyof Select<Object["table"]>]>,
    string
>;

/** The value a field's validator accepts: an identifier's plain string, else the value. */
type Accepted<Value> = Value extends Identifier<string> ? string : Value;

/** The input field with the parent. */
type ParentField<Object extends ObjectType> = "parentType" extends keyof Select<Object["table"]>
    ? "parent"
    : "parentId";

/** The value identifying an object's parent. */
type ParentValue<Object extends ObjectType> = "parentType" extends keyof Select<Object["table"]>
    ? ParentReference
    : NonNullable<Select<Object["table"]>["parentId" & keyof Select<Object["table"]>]>;

/** Whether an object may have no parent. */
type IsOrphanable<Object extends ObjectType> = null extends Select<Object["table"]>["parentId" &
    keyof Select<Object["table"]>]
    ? true
    : false;

/** A child's parent-selecting field. */
export type ParentFields<Object extends ObjectType> = "parentId" extends keyof Select<
    Object["table"]
>
    ? {
          [Field in ParentField<Object>]: IsOrphanable<Object> extends true
              ? schema.ExactOptional<
                    schema.Schema<ParentValue<Object>, Accepted<ParentValue<Object>>>
                >
              : schema.Schema<ParentValue<Object>, Accepted<ParentValue<Object>>>;
      }
    : {};

/** The parent a moved object moves to. */
export type DestinationFields<Object extends ObjectType> = "parentId" extends keyof Select<
    Object["table"]
>
    ? {
          [Field in ParentField<Object>]: IsOrphanable<Object> extends true
              ? schema.Schema<ParentValue<Object> | null, Accepted<ParentValue<Object>> | null>
              : schema.Schema<ParentValue<Object>, Accepted<ParentValue<Object>>>;
      }
    : {};

/** The field identifying a mutation: its request identifier or writer. */
export type ReplayFields<Object extends ObjectType = ObjectType> =
    Object["storage"] extends "ephemeral"
        ? { writerId: typeof WriterId }
        : { requestId: typeof RequestId.schema };

/** The keys a record may omit. */
type OptionalKeys<Record> = {
    [Key in keyof Record]-?: {} extends Pick<Record, Key> ? Key : never;
}[keyof Record];

/** The columns a creation or update writes. */
type WrittenName<Object extends ObjectType, Declared extends Method> =
    | (string extends NonNullable<Declared["fields"]>[number]
          ? Declared extends { readonly kind: "create" | "createMany" }
              ? WrittenField<Object["fields"]>
              : UpdatedField<Object["fields"]>
          : NonNullable<Declared["fields"]>[number])
    | (Declared extends { readonly kind: "create" | "createMany" }
          ? true extends NonNullable<Declared["isInternal"]>
              ? CallerField<Object["fields"]>
              : never
          : never);

/** The JSON validators of the columns a creation or update writes. */
export type WrittenFields<
    Object extends ObjectType,
    Declared extends Method,
    IsPartial extends boolean,
> = ColumnFields<Object, WrittenName<Object, Declared>, IsPartial, FilledName<Declared>>;

/** The columns a creation's handler fills where the caller leaves them out. */
type FilledName<Declared extends Method> = NonNullable<Declared["filled"]>[number];

/** One object per creation variant: the shared fields, its columns and its literal. */
export type VariantSchema<
    Object extends ObjectType,
    Declared extends Method,
    Fields,
    Variants = NonNullable<Declared["variants"]>,
> = schema.DiscriminatedUnion<
    {
        [Field in keyof Variants & string]: {
            [Value in keyof Variants[Field] & string]: schema.Object<
                Fields & { [Key in Field]: schema.Schema<Value, Value> } & RequiredFields<
                        Object,
                        VariantField<Variants[Field][Value]>
                    >
            >;
        }[keyof Variants[Field] & string];
    }[keyof Variants & string][]
>;

/** The JSON validators of the columns a write gives a value, refusing null. */
export type RequiredFields<
    Object extends ObjectType,
    Written extends string,
    Row = Insert<Object["table"]>,
> = {
    [Key in Extract<Written, keyof Row>]-?: schema.Schema<
        JsonOf<NonNullable<Row[Key]>>,
        Accepted<JsonOf<NonNullable<Row[Key]>>>
    >;
};

/** The columns one variant writes. */
type VariantField<Fields> = Fields extends readonly (infer Field extends string)[] ? Field : never;

/** The JSON validators of the given columns, each optional to a partial write. */
export type ColumnFields<
    Object extends ObjectType,
    Written extends string,
    IsPartial extends boolean,
    Filled extends string = never,
    Row = Insert<Object["table"]>,
    Name extends keyof Row = Extract<Written, keyof Row>,
> = {
    [
        Key in Name as IsPartial extends true
            ? never
            : Key extends OptionalKeys<Row> | Filled
              ? never
              : Key
    ]-?: schema.Schema<JsonOf<Row[Key]>, Accepted<JsonOf<Row[Key]>>>;
} & {
    [
        Key in Name as IsPartial extends true
            ? Key
            : Key extends OptionalKeys<Row> | Filled
              ? Key
              : never
    ]-?: schema.ExactOptional<
        schema.Schema<
            JsonOf<Exclude<Row[Key], undefined>>,
            Accepted<JsonOf<Exclude<Row[Key], undefined>>>
        >
    >;
};

/** A method's input fields. */
export type InputFields<Declared extends Method> = [NonNullable<Declared["input"]>] extends [never]
    ? {}
    : NonNullable<Declared["input"]> extends schema.Object<infer Fields>
      ? Fields
      : {};

/** A custom method's schema: routing fields joined to its input, per variant for sum types. */
export type CustomSchema<Fields extends Readonly<Record<string, schema.Schema>>, Input> = [
    NonNullable<Input>,
] extends [never]
    ? schema.Object<Fields>
    : NonNullable<Input> extends schema.DiscriminatedUnion<infer Options>
      ? schema.DiscriminatedUnion<
            (Options[number] extends infer Option
                ? Option extends schema.Object<infer Extra>
                    ? schema.Object<Fields & Extra>
                    : never
                : never)[]
        >
      : schema.Object<
            Fields & (NonNullable<Input> extends schema.Object<infer Extra> ? Extra : {})
        >;

/** One object as callers see it, in its JSON form. */
export type RowSchema<Object extends ObjectType> = schema.Object<
    RowFields<
        JsonFields<Omit<Select<Object["table"]>, SensitiveField<Object["fields"]>>>,
        GuardedField<Object["fields"]>
    > & { [Name in TextFieldName<Object["fields"]>]: schema.Schema<string> }
>;

/** Fields, each in its JSON form. */
type JsonFields<Row> = { [Name in keyof Row]: JsonOf<Row[Name]> };

/** The validators of a row's fields, the guarded ones accepting omission. */
type RowFields<Row, Guarded> = {
    [Name in Exclude<keyof Row, Guarded>]: schema.Schema<Row[Name]>;
} & {
    [Name in Extract<keyof Row, Guarded>]: schema.ExactOptional<schema.Schema<Row[Name]>>;
};

/** The procedures of an object's methods: their routes and input and output schemas. */
const ObjectProcedure = {
    /** Build an object's row as callers see it: guarded fields optional, texts as strings. */
    rowSchema(object: ObjectType): ObjectSchema["row"] {
        // leave out the sensitive columns
        const columns = schema.object(jsonShape(object.table[TABLE], "select"));
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
    },

    /** Build the reader of the written columns' schemas. */
    writtenSchema(object: ObjectType): ObjectSchema["written"] {
        // read the written columns' JSON schemas
        const inserted = jsonShape(object.table[TABLE], "insert");

        return (names, isPartial) =>
            Object.fromEntries(
                (names ?? (isPartial ? object.updated : object.written)).map((name) => {
                    // take each written column's schema
                    const field = object.fields[name];
                    const value = object.table[TABLE].columns[name]?.definition.json;
                    const isRequired = field?.required === true && field.access?.read !== undefined;

                    // require a guarded field its nullable column would leave optional
                    const required = field?.initial === undefined ? value : value?.exactOptional();
                    const declared = isRequired ? required : inserted[name];
                    if (declared === undefined) {
                        throw new TypeError(`${object.name} has no writable column ${name}`);
                    }

                    return [name, isPartial ? declared.exactOptional() : declared];
                }),
            );
    },

    /** Build the parent a created object belongs to and the parent a moved object moves to. */
    parentShapes(object: ObjectType): Pick<ObjectSchema, "parent" | "destination"> {
        // read the parent columns of a nested object
        const nesting = object.parent;
        const parent = nesting && ObjectProcedure.parentSchema(nesting.object);

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
    },

    /** Build the schema identifying an object's parent. */
    parentSchema(parent: ObjectType | "any"): InputObject["shape"] {
        return parent === "any" ? { parent: ParentReference } : { parentId: parent.idSchema };
    },

    /** Declare one derived procedure with its access, route, input and output. */
    procedure<Input extends schema.Schema, Output extends ProcedureOutput>(
        access: ReturnType<typeof defineProcedure>,
        route: { readonly method: Route["method"]; readonly path: `/${string}` },
        input: Input,
        output: Output,
    ) {
        return access.route(route).input(input).output(output);
    },
};

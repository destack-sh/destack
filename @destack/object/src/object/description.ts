import { Policy, PolicySubject } from "@destack/access";
import {
    check,
    Condition,
    ForeignKey,
    Index,
    TABLE,
    Unique,
    uniqueIndex,
    index,
    REFERENCE_ACTIONS,
    type Column,
    type ColumnMap,
    type SQL,
    type Table,
    type TableConstraint,
} from "@destack/db";
import { PackageId } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import { ResourceDeclaration, ResourceKind } from "@destack/resource";
import {
    defineSchema,
    fromJsonSchema,
    present,
    schema,
    toJsonSchema,
    type JsonObject,
    type JsonValue,
} from "@destack/schema";
import { ObjectTypeReference } from "@destack/sync";
import { field, Field, FIELD_MODIFIERS } from "../field/field.ts";
import type { FileBucket } from "../field/file.ts";
import { Method } from "../method/method.ts";
import { method } from "../method/record.ts";
import { ObjectType, type ObjectDefinition } from "./object.ts";
import { INTRINSIC } from "./intrinsic.ts";

/** The referential actions a foreign key takes. */
const ReferenceActionName = schema.enum(REFERENCE_ACTIONS);

/** A declared constraint as data, a foreign key's referenced table and columns by SQL name. */
const ConstraintDescription = schema.discriminatedUnion("kind", [
    schema.object({
        kind: schema.literal("check"),
        name: schema.string(),
        condition: Condition.schema,
    }),
    schema.object({
        kind: schema.literal("index"),
        name: schema.string(),
        isUnique: schema.boolean(),
        columns: schema.array(schema.string()),
        where: Condition.schema.exactOptional(),
        isNullsNotDistinct: schema.literal(true).exactOptional(),
    }),
    schema.object({
        kind: schema.literal("unique"),
        name: schema.string().exactOptional(),
        columns: schema.array(schema.string()),
    }),
    schema.object({
        kind: schema.literal("foreignKey"),
        name: schema.string().exactOptional(),
        columns: schema.array(schema.string()),
        references: schema.object({
            table: schema.string(),
            columns: schema.array(schema.string()),
        }),
        onDelete: ReferenceActionName.exactOptional(),
        onUpdate: ReferenceActionName.exactOptional(),
    }),
]);
/** A declared constraint as data. */
type ConstraintDescription = schema.Infer<typeof ConstraintDescription>;

/** The definition keys whose values may be lazy references, `*` for any key and `[]` for a list's entries. */
const LAZY_PATHS: readonly (readonly string[])[] = [
    ["projections"],
    ["indexes", "*", "across"],
    ["projected", "from"],
];

/** The builders whose function arguments are lazy references, such as `field.reference("note", () => note)`. */
const LAZY_BUILDERS: ReadonlySet<string> = new Set(["reference"]);

/** Where a value sits in a definition being described. */
interface Position {
    /** The path messages show, such as `task.indexes.name.across`. */
    readonly path: string;
    /** The keys from the definition's root, absent inside a builder's arguments. */
    readonly keys: readonly string[] | undefined;
    /** Whether the value may be a lazy reference to types or policies declared later. */
    readonly isLazy: boolean;
}

/** A builder call as written: the builder's name and its arguments. */
export interface BuilderCall {
    /** The builder, a function of `field` or `method`, or a modifier of what an earlier call built. */
    readonly builder: string;
    /** The arguments, in order. */
    readonly arguments: readonly unknown[];
}

/** A declaration a description may refer to: an object type, a policy, a resource kind or a bucket. */
export type KnownDeclaration = ObjectType | Policy | ResourceKind | FileBucket;

/** An object type as data: its package and its definition as written, its special values tagged. */
const objectDescription = defineSchema(
    schema.object({
        /** The declaring package. */
        package: schema.object({
            /** The package's identifier. */
            id: PackageId,
            /** The package's name. */
            name: schema.string().min(1),
            /** The package's version. */
            version: schema.string().min(1),
        }),
        /** The definition as written, special values tagged. */
        definition: schema.record(schema.string(), schema.json()),
    }),
);

/** An object type as data, with the verbs reading a description. */
export const ObjectDescription = Object.assign(objectDescription, {
    /** Key a description's type by its policy: its own, or the principal kind it represents. */
    key(description: ObjectDescription): string {
        const { represents, name } = description.definition;
        const reference =
            represents === undefined
                ? { packageId: description.package.id, type: schema.string().parse(name) }
                : schema.object({ $policy: ObjectTypeReference }).parse(represents).$policy;

        return ObjectTypeReference.key(reference);
    },

    /** List the keys of the types and policies a description needs now that the index lacks. */
    dependencies(description: ObjectDescription, known: DeclarationIndex): string[] {
        // collect each type and policy reference outside lazy ones the index lacks
        const keys: string[] = [];
        const visit = (value: JsonValue): void => {
            if (value === null || typeof value !== "object") {
                return;
            } else if (isList(value)) {
                value.forEach(visit);
            } else if ("$object" in value || "$policy" in value) {
                const object = value["$object"];
                const key = referenceKey(object ?? value["$policy"]);
                const declarations = object === undefined ? known.policies : known.objects;
                if (!declarations.has(key)) {
                    keys.push(key);
                }
            } else if (!("$lazy" in value)) {
                Object.values(value).forEach(visit);
            }
        };
        visit(description.definition);

        return keys;
    },

    /** Decode a description's definition against the known declarations, its methods built. */
    decode(description: ObjectDescription, known: DeclarationIndex): ObjectDefinition {
        return definitionOf(decode(description.definition, known));
    },
});
/** An object type as data. */
export type ObjectDescription = schema.Infer<typeof objectDescription>;

/** Encode an object type as data, refusing an intrinsic type. */
export function encodeType(type: ObjectType): ObjectDescription {
    // refuse an intrinsic type, whose table another package keeps
    const { source } = type;
    if (INTRINSIC in source) {
        throw new TypeError(`object type ${type.name} maps a table another package keeps`);
    }

    // tag the definition's special values, rendering its constraints over the type's columns
    const columns = type.table[TABLE].columns;
    const { constraints, ...rest } = source;
    const encoded = encodeObject(rest, { path: type.name, keys: [], isLazy: false });
    const definition =
        constraints === undefined
            ? encoded
            : {
                  ...encoded,
                  constraints: {
                      $constraints: constraints(columns).map((each) =>
                          describeConstraint(each, columns),
                      ),
                  },
              };

    return {
        package: {
            id: type.package.id,
            name: type.package.name,
            version: type.package.version,
        },
        definition,
    };
}

/** The declarations descriptions refer to, by their declaring package and name. */
export class DeclarationIndex {
    /** The object types, by reference key. */
    readonly objects = new Map<string, ObjectType>();
    /** The policies, the object types' own included, by reference key. */
    readonly policies = new Map<string, Policy>();
    /** The resource kinds, by package and name. */
    readonly kinds = new Map<string, ResourceKind>();
    /** The buckets, by package and name. */
    readonly buckets = new Map<string, FileBucket>();

    /** Index known declarations. */
    static of(known: Iterable<KnownDeclaration>): DeclarationIndex {
        const declarations = new DeclarationIndex();
        for (const declaration of known) {
            declarations.add(declaration);
        }

        return declarations;
    }

    /** Add a declaration, such as an object type a description just declared. */
    add(declaration: KnownDeclaration): void {
        if (declaration instanceof ObjectType) {
            const key = ObjectTypeReference.key(declaration.typeReference);
            this.objects.set(key, declaration);
            this.policies.set(key, declaration.policy);
        } else if (declaration instanceof Policy) {
            this.policies.set(ObjectTypeReference.key(declaration.typeReference), declaration);
        } else if (declaration instanceof ResourceKind) {
            this.kinds.set(keyOf(DeclarationReference.of(declaration)), declaration);
        } else {
            this.buckets.set(keyOf(DeclarationReference.of(declaration)), declaration);
        }
    }

    /** Read a declaration by key, refusing one the index lacks. */
    require<Value>(declarations: ReadonlyMap<string, Value>, key: string, noun: string): Value {
        const declaration = declarations.get(key);
        if (declaration === undefined) {
            throw new TypeError(`a description refers to ${noun} ${key}, which is not known`);
        }

        return declaration;
    }

    /** Read the table of a known object type by its SQL name. */
    table(name: string): Table {
        const table = [...this.objects.values()]
            .flatMap((type) => type.tables)
            .find((each) => each[TABLE].sqlName === name);
        if (table === undefined) {
            throw new TypeError(`a description refers to table ${name}, which is not known`);
        }

        return table;
    }
}

/** Read a decoded definition. */
function definitionOf(decoded: unknown): ObjectDefinition;
/**
 * Read a decoded definition as the definition it encodes.
 *
 * @construct a description's definition encodes a definition as written, which decoding rebuilds value for value.
 */
function definitionOf(decoded: unknown): unknown {
    schema.looseObject({ name: schema.string(), plural: schema.string() }).parse(decoded);

    return decoded;
}

/** Tag a plain object's special values, refusing keys that start like a tag. */
function encodeObject(value: object, position: Position): JsonObject {
    // refuse keys that start like a tag
    const entries = Object.entries(value).filter(([, entry]) => entry !== undefined);
    if (entries.some(([key]) => key.startsWith("$"))) {
        throw new TypeError(
            `${position.path} has a key starting with $, which descriptions keep for tags`,
        );
    }

    // tag each present entry
    return Object.fromEntries(
        entries.map(([key, entry]) => [
            key,
            encode(entry, entryOf(position, key, `${position.path}.${key}`)),
        ]),
    );
}

/** Tag a value's types, fields, methods, schemas, kinds and buckets as data, refusing other code. */
function encode(value: unknown, position: Position): JsonValue {
    // keep plain data
    if (
        value === null ||
        typeof value === "string" ||
        typeof value === "number" ||
        typeof value === "boolean"
    ) {
        return value;
    } else if (Array.isArray(value)) {
        return value.map((entry, offset) =>
            encode(entry, entryOf(position, "[]", `${position.path}[${offset}]`)),
        );
    }
    // tag declarations by reference
    else if (value instanceof ObjectType) {
        return { $object: value.typeReference };
    } else if (value instanceof Policy) {
        return { $policy: value.typeReference };
    } else if (value instanceof PolicySubject) {
        return encode(value.type(), position);
    } else if (value instanceof ResourceKind) {
        return { $kind: DeclarationReference.of(value) };
    } else if (value instanceof ResourceDeclaration) {
        return { $bucket: DeclarationReference.of(value) };
    }
    // tag fields, methods and schemas by how they were built
    else if (value instanceof Field) {
        return { $field: value.describe().map((call) => encodeCall(call, position)) };
    } else if (Method.is(value)) {
        return { $method: [encodeCall(methodSourceOf(value, position), position)] };
    } else if (value instanceof schema.Schema) {
        return { $schema: toJsonSchema(value) };
    }
    // tag a lazy reference where the definition takes one
    else if (typeof value === "function" && position.isLazy) {
        return lazyOf(value, position);
    }
    // tag a plain object's entries
    else if (typeof value === "object" && Object.getPrototypeOf(value) === Object.prototype) {
        return encodeObject(value, position);
    }
    // refuse any other value, which is code
    else {
        throw new TypeError(`${position.path} is code, which a description cannot keep`);
    }
}

/** Tag a builder call, its arguments tagged and trailing absent ones left out. */
function encodeCall(call: BuilderCall, position: Position): JsonObject {
    // leave out trailing absent arguments
    const written = [...call.arguments];
    while (written.length > 0 && written.at(-1) === undefined) {
        written.pop();
    }

    // tag each argument, a lazy reference where the builder takes one
    return {
        builder: call.builder,
        arguments: written.map((argument, offset) =>
            encode(argument, {
                path: `${position.path}.${call.builder}[${offset}]`,
                keys: undefined,
                isLazy: LAZY_BUILDERS.has(call.builder),
            }),
        ),
    };
}

/** Read the builder call of a declared method, refusing one with a handler. */
function methodSourceOf(declared: Method, position: Position): BuilderCall {
    if (declared.source === undefined) {
        throw new TypeError(
            `${position.path} has a handler; attach it with .implement({ methods })`,
        );
    }

    return declared.source;
}

/** Tag a lazy reference by the types or policies it returns, refusing one returning anything else. */
function lazyOf(value: Function, position: Position): JsonValue {
    // resolve the reference, refusing one returning anything but types or policies
    const resolved: unknown = Reflect.apply(value, undefined, []);
    const returned = Array.isArray(resolved) ? resolved : [resolved];
    if (
        returned.length === 0 ||
        !returned.every((each) => each instanceof ObjectType || each instanceof Policy)
    ) {
        throw new TypeError(`${position.path} returns no object types or policies`);
    }

    return { $lazy: encode(resolved, { ...position, isLazy: false }) };
}

/** Read the position of an entry of a value: its path, its definition keys and whether it takes a lazy reference. */
function entryOf(position: Position, key: string, path: string): Position {
    const keys = position.keys === undefined ? undefined : [...position.keys, key];
    const isLazy =
        keys !== undefined &&
        LAZY_PATHS.some(
            (pattern) =>
                pattern.length === keys.length &&
                pattern.every((each, offset) => each === "*" || each === keys[offset]),
        );

    return { path, keys, isLazy };
}

/** Rebuild a description's values, resolving each tag. */
function decode(value: JsonValue, known: DeclarationIndex): unknown {
    // keep plain values
    if (value === null || typeof value !== "object") {
        return value;
    }
    // rebuild each entry of a list
    else if (isList(value)) {
        return value.map((entry) => decode(entry, known));
    }

    // resolve a tag, or rebuild a plain object's entries
    const tag = Object.keys(value).find((key) => key.startsWith("$"));

    return tag === undefined
        ? Object.fromEntries(
              Object.entries(value).map(([key, entry]) => [key, decode(entry, known)]),
          )
        : decodeTag(tag, value[tag], known);
}

/** Decide whether a JSON value is a list. */
function isList(value: readonly JsonValue[] | JsonObject): value is readonly JsonValue[] {
    return Array.isArray(value);
}

/** Resolve one tagged value. */
function decodeTag(tag: string, value: JsonValue | undefined, known: DeclarationIndex): unknown {
    switch (tag) {
        case "$object":
            return known.require(known.objects, referenceKey(value), "object type");
        case "$policy":
            return known.require(known.policies, referenceKey(value), "policy");
        case "$kind":
            return known.require(
                known.kinds,
                keyOf(DeclarationReference.parse(value)),
                "resource kind",
            );
        case "$bucket":
            return known.require(known.buckets, keyOf(DeclarationReference.parse(value)), "bucket");
        case "$lazy":
            return () => decode(present(value, "the lazy declaration"), known);
        case "$schema":
            return fromJsonSchema(schema.record(schema.string(), schema.json()).parse(value));
        case "$field":
            return replayed(field, FIELD_MODIFIERS, BuilderCalls.parse(value), known);
        case "$method":
            return replayed(method, [], BuilderCalls.parse(value), known);
        case "$constraints": {
            const constraints = schema.array(ConstraintDescription).parse(value);
            return (columns: ColumnMap) =>
                constraints.map((each) => constraintOf(each, columns, known));
        }
        default:
            throw new TypeError(`a description has an unknown tag ${tag}`);
    }
}

/** A chain of builder calls as data. */
const BuilderCalls = schema
    .array(
        schema.object({ builder: schema.string().min(1), arguments: schema.array(schema.json()) }),
    )
    .min(1);

/** Replay a chain of builder calls: a builder of the given ones, then modifiers of what it built. */
function replayed(
    builders: object,
    modifiers: readonly string[],
    calls: schema.Infer<typeof BuilderCalls>,
    known: DeclarationIndex,
): unknown {
    // call the builder, then each modifier on what it built
    const [first, ...rest] = calls;
    let built = invoke(builders, present(first, "the first builder call"), known);
    for (const each of rest) {
        if (!modifiers.includes(each.builder)) {
            throw new TypeError(
                `a description modifies with ${each.builder}, which is no modifier`,
            );
        }
        built = invoke(built, each, known);
    }

    return built;
}

/** Call a builder its owner has, with the arguments decoded, refusing any other name. */
function invoke(
    owner: unknown,
    built: schema.Infer<typeof BuilderCalls>[number],
    known: DeclarationIndex,
): unknown {
    const builder: unknown =
        typeof owner === "object" && owner !== null ? Reflect.get(owner, built.builder) : undefined;
    if (typeof builder !== "function" || built.builder === "constructor") {
        throw new TypeError(`a description calls ${built.builder}, which is no builder`);
    }

    return Reflect.apply(
        builder,
        owner,
        built.arguments.map((argument) => decode(argument, known)),
    );
}

/** Describe a declared constraint, a foreign key's referenced columns by SQL name. */
function describeConstraint(
    constraint: TableConstraint,
    columns: ColumnMap,
): ConstraintDescription {
    if (constraint.kind === "check") {
        return {
            kind: "check",
            name: constraint.name,
            condition: Condition.schema.parse(constraint.condition),
        };
    } else if (constraint instanceof Index) {
        return {
            kind: "index",
            name: constraint.name,
            isUnique: constraint.isUnique,
            columns: constraint.columns.map((column) => {
                if (!isColumn(column)) {
                    throw new TypeError(
                        `index ${constraint.name} has an SQL expression, which a description cannot keep`,
                    );
                }

                return propertyOf(column, columns);
            }),
            ...(constraint.predicate === undefined
                ? {}
                : { where: Condition.schema.parse(constraint.predicate) }),
            ...(constraint.isNullsNotDistinct ? { isNullsNotDistinct: true } : {}),
        };
    } else if (constraint instanceof Unique) {
        return {
            kind: "unique",
            ...(constraint.name === undefined ? {} : { name: constraint.name }),
            columns: constraint.columns.map((column) => propertyOf(column, columns)),
        };
    }

    return {
        kind: "foreignKey",
        ...(constraint.name === undefined ? {} : { name: constraint.name }),
        columns: constraint.columns.map((column) => propertyOf(column, columns)),
        references: {
            table: constraint.references,
            columns: constraint.foreignColumns.map((column) => column.definition.name),
        },
        ...constraint.actions,
    };
}

/** Rebuild a declared constraint over a table's columns, against the known declarations. */
function constraintOf(
    description: ConstraintDescription,
    columns: ColumnMap,
    known: DeclarationIndex,
): TableConstraint {
    switch (description.kind) {
        case "check":
            return check(description.name, description.condition);
        case "index": {
            const built = (description.isUnique ? uniqueIndex : index)(description.name);
            const [first, ...rest] = description.columns.map((column) => columnOf(column, columns));
            if (first === undefined) {
                throw new TypeError(`index ${description.name} has no column`);
            }
            const selected = built.on(first, ...rest);
            const filtered =
                description.where === undefined ? selected : selected.where(description.where);

            return description.isNullsNotDistinct === true ? filtered.nullsNotDistinct() : filtered;
        }
        case "unique": {
            const [first, ...rest] = description.columns.map((column) => columnOf(column, columns));
            if (first === undefined) {
                throw new TypeError("a unique constraint keeps no column");
            }

            return new Unique(description.name).on(first, ...rest);
        }
        case "foreignKey": {
            const referenced = known.table(description.references.table);
            const { onDelete, onUpdate } = description;

            return new ForeignKey(
                {
                    ...(description.name === undefined ? {} : { name: description.name }),
                    columns: description.columns.map((column) => columnOf(column, columns)),
                    foreignColumns: description.references.columns.map((name) =>
                        sqlColumnOf(name, referenced),
                    ),
                },
                {
                    ...(onDelete === undefined ? {} : { onDelete }),
                    ...(onUpdate === undefined ? {} : { onUpdate }),
                },
            );
        }
    }
}

/** Read a table's column by its SQL name. */
function sqlColumnOf(name: string, table: Table): Column {
    const column = Object.values(table[TABLE].columns).find(
        (each) => each.definition.name === name,
    );
    if (column === undefined) {
        throw new TypeError(`table ${table[TABLE].sqlName} has no column ${name}`);
    }

    return column;
}

/** Decide whether an index entry is a column rather than an expression. */
function isColumn(entry: Column | SQL): entry is Column {
    return "definition" in entry;
}

/** Read the property a table keeps a column under. */
function propertyOf(column: Column, columns: ColumnMap): string {
    const entry = Object.entries(columns).find(([, each]) => each === column);
    if (entry === undefined) {
        throw new TypeError(`column ${column.definition.name} is not one of the table's`);
    }

    return entry[0];
}

/** Read a table's column by property. */
function columnOf(property: string, columns: ColumnMap): Column {
    const column = columns[property];
    if (column === undefined) {
        throw new TypeError(`the table has no column ${property}`);
    }

    return column;
}

/** Key a declaration by its declaring package and name. */
function keyOf(reference: DeclarationReference): string {
    return JSON.stringify([reference.packageId, reference.name]);
}

/** Key an object type or policy reference. */
function referenceKey(value: JsonValue | undefined): string {
    return ObjectTypeReference.key(ObjectTypeReference.parse(value));
}

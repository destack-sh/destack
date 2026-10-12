import { Policy, PolicySubject } from "@destack/access";
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

/** The definition keys that may have lazy references, `*` for any and `[]` for list entries. */
const LAZY_PATHS: readonly (readonly string[])[] = [
    ["projections"],
    ["indexes", "*", "across"],
    ["projected", "from"],
    ["constraints", "*", "references", "object"],
];

/** The builders taking lazy function references, such as `field.reference("note", () => note)`. */
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
    /** The builder: a `field` or `method` function, or a modifier of an earlier result. */
    readonly builder: string;
    /** The arguments, in order. */
    readonly arguments: readonly unknown[];
}

/** A declaration a description may refer to: an object type, policy, resource kind or bucket. */
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

    // tag the definition's special values
    const definition = encodeObject(source, { path: type.name, keys: [], isLazy: false });

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
}

/** Read a decoded definition. */
function definitionOf(decoded: unknown): ObjectDefinition;
/**
 * Read a decoded definition as the definition it encodes.
 *
 * @construct a description encodes a definition as written, which decoding rebuilds exactly.
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

/** Tag a value's types, fields, methods, schemas, kinds and buckets, refusing other code. */
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

/** Tag a lazy reference by its returned types or policies, refusing other returns. */
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

/** Read an entry's position: its path, definition keys and whether it takes a lazy reference. */
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

/** Replay a chain of builder calls: one of the given builders, then modifiers of its result. */
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

/** Key a declaration by its declaring package and name. */
function keyOf(reference: DeclarationReference): string {
    return JSON.stringify([reference.packageId, reference.name]);
}

/** Key an object type or policy reference. */
function referenceKey(value: JsonValue | undefined): string {
    return ObjectTypeReference.key(ObjectTypeReference.parse(value));
}

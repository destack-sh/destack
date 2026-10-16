import { INTRINSIC_POLICIES, Policy, PolicySubject } from "@destack/access";
import { DeclarationName, Package, PackageId } from "@destack/package";
import { NO_STATE, ResourceDeclaration, ResourceKind } from "@destack/resource";
import {
    defineSchema,
    fromJsonSchema,
    present,
    schema,
    toJsonSchema,
    type JsonObject,
    JsonValue,
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
    ["indexes", "*", "across"],
    ["constraints", "*", "references", "object"],
];

/** The builders taking lazy function references, such as `field.reference(() => note)`. */
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

/** A declaration a description may refer to: an object type or a policy. */
export type KnownDeclaration = ObjectType | Policy;

/** A JSON Schema as a description holds it. */
const DescribedSchema = schema.jsonObject();

/** A resource kind as a description holds it: its package, name and schemas. */
const DescribedKind = schema.object({
    /** The declaring package. */
    package: Package,
    /** The kind's name. */
    name: DeclarationName,
    /** The schema of a declaration's specification. */
    spec: DescribedSchema,
    /** The schema of the desired state its providers reconcile, null for a kind without one. */
    state: DescribedSchema.nullable(),
});

/** A bucket as a description holds it: its package, name and specification. */
const DescribedBucket = schema.object({
    /** The declaring package. */
    package: Package,
    /** The bucket's name. */
    name: DeclarationName,
    /** The bucket's specification. */
    spec: schema.jsonObject(),
});

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
        definition: schema.jsonObject(),
    }),
);

/** A declaration value as description data: tagged, encoded and decoded. */
const DescribedValue = {
    /** Tag a plain object's special values, refusing keys that start like a tag. */
    encodeObject(value: object, position: Position): JsonObject {
        // refuse keys that start like a tag
        const entries = Object.entries(value).filter(([, entry]) => entry !== undefined);
        if (entries.some(([key]) => key.startsWith("$"))) {
            throw new TypeError(
                `${position.path} has a key starting with $: descriptions reserve it for tags`,
            );
        }

        // tag each present entry
        return Object.fromEntries(
            entries.map(([key, entry]) => [
                key,
                DescribedValue.encode(
                    entry,
                    DescribedValue.readEntry(position, key, `${position.path}.${key}`),
                ),
            ]),
        );
    },

    /** Tag a value's types, fields, methods, schemas, kinds and buckets, refusing other code. */
    encode(value: unknown, position: Position): JsonValue {
        // take plain data as it is
        if (
            value === null ||
            typeof value === "string" ||
            typeof value === "number" ||
            typeof value === "boolean"
        ) {
            return value;
        } else if (Array.isArray(value)) {
            return value.map((entry, offset) =>
                DescribedValue.encode(
                    entry,
                    DescribedValue.readEntry(position, "[]", `${position.path}[${offset}]`),
                ),
            );
        }
        // tag types and policies by reference, kinds and buckets as data
        else if (value instanceof ObjectType) {
            return { $object: value.typeReference };
        } else if (value instanceof Policy) {
            return { $policy: value.typeReference };
        } else if (value instanceof PolicySubject) {
            return DescribedValue.encode(value.type(), position);
        } else if (value instanceof ResourceKind) {
            return { $kind: DescribedValue.encodeKind(value) };
        } else if (value instanceof ResourceDeclaration) {
            return { $bucket: DescribedValue.encodeBucket(value) };
        }
        // tag fields, methods and schemas by how they were built
        else if (value instanceof Field) {
            return {
                $field: value.describe().map((call) => DescribedValue.encodeCall(call, position)),
            };
        } else if (Method.is(value)) {
            return {
                $method: [
                    DescribedValue.encodeCall(
                        DescribedValue.readMethodSource(value, position),
                        position,
                    ),
                ],
            };
        } else if (value instanceof schema.Schema) {
            return { $schema: JsonValue.object(toJsonSchema(value)) };
        }
        // tag a lazy reference where the definition takes one
        else if (typeof value === "function" && position.isLazy) {
            return DescribedValue.tagLazy(value, position);
        }
        // tag a plain object's entries
        else if (typeof value === "object" && Object.getPrototypeOf(value) === Object.prototype) {
            return DescribedValue.encodeObject(value, position);
        }
        // refuse any other value, which is code
        else {
            throw new TypeError(`${position.path} is code: a description cannot store it`);
        }
    },

    /** Tag a builder call, its arguments tagged and trailing absent ones left out. */
    encodeCall(call: BuilderCall, position: Position): JsonObject {
        // leave out trailing absent arguments
        const written = [...call.arguments];
        while (written.length > 0 && written.at(-1) === undefined) {
            written.pop();
        }

        // tag each argument, a lazy reference where the builder takes one
        return {
            builder: call.builder,
            arguments: written.map((argument, offset) =>
                DescribedValue.encode(argument, {
                    path: `${position.path}.${call.builder}[${offset}]`,
                    keys: undefined,
                    isLazy: LAZY_BUILDERS.has(call.builder),
                }),
            ),
        };
    },

    /** Read the builder call of a declared method, refusing one with a handler. */
    readMethodSource(declared: Method, position: Position): BuilderCall {
        if (declared.source === undefined) {
            throw new TypeError(
                `${position.path} has a handler; attach it with .implement({ methods })`,
            );
        }

        return declared.source;
    },

    /** Tag a lazy reference by its returned types or policies, refusing other returns. */
    tagLazy(value: Function, position: Position): JsonValue {
        // resolve the reference, refusing one returning anything but types or policies
        const resolved: unknown = Reflect.apply(value, undefined, []);
        const returned = Array.isArray(resolved) ? resolved : [resolved];
        if (
            returned.length === 0 ||
            !returned.every((each) => each instanceof ObjectType || each instanceof Policy)
        ) {
            throw new TypeError(`${position.path} returns no object types or policies`);
        }

        return { $lazy: DescribedValue.encode(resolved, { ...position, isLazy: false }) };
    },

    /** Read an entry's position: its path, definition keys and whether it is lazy. */
    readEntry(position: Position, key: string, path: string): Position {
        const keys = position.keys === undefined ? undefined : [...position.keys, key];
        const isLazy =
            keys !== undefined &&
            LAZY_PATHS.some(
                (pattern) =>
                    pattern.length === keys.length &&
                    pattern.every((each, offset) => each === "*" || each === keys[offset]),
            );

        return { path, keys, isLazy };
    },

    /** Rebuild a description's values, resolving each tag. */
    decode(value: JsonValue, known: DeclarationIndex): unknown {
        // take plain values as they are
        if (value === null || typeof value !== "object") {
            return value;
        }
        // rebuild each entry of a list
        else if (DescribedValue.isList(value)) {
            return value.map((entry) => DescribedValue.decode(entry, known));
        }

        // resolve a tag, or rebuild a plain object's entries
        const tag = Object.keys(value).find((key) => key.startsWith("$"));

        return tag === undefined
            ? Object.fromEntries(
                  JsonValue.entries(value).map(([key, entry]) => [
                      key,
                      DescribedValue.decode(entry, known),
                  ]),
              )
            : DescribedValue.decodeTag(tag, value[tag], known);
    },

    /** Decide whether a JSON value is a list. */
    isList(value: readonly JsonValue[] | JsonObject): value is readonly JsonValue[] {
        return Array.isArray(value);
    },

    /** Resolve one tagged value. */
    decodeTag(tag: string, value: JsonValue | undefined, known: DeclarationIndex): unknown {
        switch (tag) {
            case "$object":
                return known.require(
                    known.objects,
                    DescribedValue.referenceKey(value),
                    "object type",
                );
            case "$policy":
                return known.require(known.policies, DescribedValue.referenceKey(value), "policy");
            case "$kind":
                return DescribedValue.decodeKind(DescribedKind.parse(value));
            case "$bucket":
                return DescribedValue.decodeBucket(DescribedBucket.parse(value));
            case "$lazy":
                return () => DescribedValue.decode(present(value, "the lazy declaration"), known);
            case "$schema":
                return fromJsonSchema(DescribedSchema.parse(value));
            case "$field":
                return DescribedValue.replayed(
                    field,
                    FIELD_MODIFIERS,
                    BuilderCalls.parse(value),
                    known,
                );
            case "$method":
                return DescribedValue.replayed(method, [], BuilderCalls.parse(value), known);
            default:
                throw new TypeError(`a description has an unknown tag ${tag}`);
        }
    },

    /** Replay a chain of builder calls: one of the given builders, then modifiers of its result. */
    replayed(
        builders: object,
        modifiers: readonly string[],
        calls: schema.Infer<typeof BuilderCalls>,
        known: DeclarationIndex,
    ): unknown {
        // call the builder, then each modifier on what it built
        const [first, ...rest] = calls;
        let built = DescribedValue.invoke(
            builders,
            present(first, "the first builder call"),
            known,
        );
        for (const each of rest) {
            if (!modifiers.includes(each.builder)) {
                throw new TypeError(
                    `a description modifies with ${each.builder}, which is not a modifier`,
                );
            }
            built = DescribedValue.invoke(built, each, known);
        }

        return built;
    },

    /** Call a builder its owner has, with the arguments decoded, refusing any other name. */
    invoke(
        owner: unknown,
        built: schema.Infer<typeof BuilderCalls>[number],
        known: DeclarationIndex,
    ): unknown {
        const builder: unknown =
            typeof owner === "object" && owner !== null
                ? Reflect.get(owner, built.builder)
                : undefined;
        if (typeof builder !== "function" || built.builder === "constructor") {
            throw new TypeError(`a description calls ${built.builder}, which is not a builder`);
        }

        return Reflect.apply(
            builder,
            owner,
            built.arguments.map((argument) => DescribedValue.decode(argument, known)),
        );
    },

    /** List the type and policy references in a description value that the index lacks. */
    missing(
        value: JsonValue,
        known: DeclarationIndex,
        isLazyFollowed: boolean,
    ): ObjectTypeReference[] {
        // read nothing from plain values, and each entry of a list
        if (value === null || typeof value !== "object") {
            return [];
        } else if (DescribedValue.isList(value)) {
            return value.flatMap((entry) => DescribedValue.missing(entry, known, isLazyFollowed));
        }
        // pass a type or policy reference the index lacks
        else if ("$object" in value || "$policy" in value) {
            const object = value["$object"];
            const reference = ObjectTypeReference.parse(object ?? value["$policy"]);
            const declarations = object === undefined ? known.policies : known.objects;

            return declarations.has(ObjectTypeReference.key(reference)) ? [] : [reference];
        }
        // follow a lazy reference only when asked, and every other entry
        else if ("$lazy" in value && !isLazyFollowed) {
            return [];
        }

        return JsonValue.values(value).flatMap((entry) =>
            DescribedValue.missing(entry, known, isLazyFollowed),
        );
    },

    /** Describe a resource kind: its package, name and JSON Schemas. */
    encodeKind(
        kind: Pick<ResourceKind, "package" | "name" | "spec" | "state" | "isStateful">,
    ): JsonObject {
        return {
            package: DescribedValue.readPackage(kind.package),
            name: kind.name,
            spec: JsonValue.object(toJsonSchema(kind.spec)),
            state: kind.isStateful ? JsonValue.object(toJsonSchema(kind.state)) : null,
        };
    },

    /** Rebuild a resource kind a description holds. */
    decodeKind(described: schema.Infer<typeof DescribedKind>): ResourceKind {
        const { package: owner, name, spec, state } = described;

        return new ResourceKind(
            owner,
            name,
            fromJsonSchema(spec),
            state === null ? NO_STATE : fromJsonSchema(state),
        );
    },

    /** Describe a bucket as data: its package, name and specification. */
    encodeBucket(bucket: {
        readonly package: Package;
        readonly name: string;
        readonly spec: Readonly<Record<string, JsonValue>>;
    }): JsonObject {
        return {
            package: DescribedValue.readPackage(bucket.package),
            name: bucket.name,
            spec: bucket.spec,
        };
    },

    /** Rebuild a bucket a description holds. */
    decodeBucket(described: schema.Infer<typeof DescribedBucket>): FileBucket {
        const { package: owner, ...bucket } = described;

        return new ResourceDeclaration(owner, { ...bucket, kind: "bucket" });
    },

    /** Read only a package's identity. */
    readPackage(owner: Package): JsonObject {
        return { id: owner.id, name: owner.name, version: owner.version };
    },

    /** Key an object type or policy reference. */
    referenceKey(value: JsonValue | undefined): string {
        return ObjectTypeReference.key(ObjectTypeReference.parse(value));
    },
};

/** An object type as data, with the verbs reading a description. */
export const ObjectDescription = Object.assign(objectDescription, {
    /** Encode an object type as data, refusing an intrinsic type. */
    of(type: ObjectType): ObjectDescription {
        // refuse an intrinsic type: another package owns its table
        const { source } = type;
        if (INTRINSIC in source) {
            throw new TypeError(`object type ${type.name} maps a table of another package`);
        }

        // tag the definition's special values
        const definition = DescribedValue.encodeObject(source, {
            path: type.name,
            keys: [],
            isLazy: false,
        });

        return {
            package: {
                id: type.package.id,
                name: type.package.name,
                version: type.package.version,
            },
            definition,
        };
    },

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
        return DescribedValue.missing(description.definition, known, false).map((reference) =>
            ObjectTypeReference.key(reference),
        );
    },

    /** List the types and policies a description refers to that the index lacks, lazy ones too. */
    references(description: ObjectDescription, known: DeclarationIndex): ObjectTypeReference[] {
        const references = DescribedValue.missing(description.definition, known, true).map(
            (reference) => [ObjectTypeReference.key(reference), reference] as const,
        );

        return [...new Map(references).values()];
    },

    /** Decode a description's definition against the known declarations, its methods built. */
    decode(description: ObjectDescription, known: DeclarationIndex): ObjectDefinition {
        return readDefinition(DescribedValue.decode(description.definition, known));
    },
});
/** An object type as data. */
export type ObjectDescription = schema.Infer<typeof objectDescription>;

/** The types and policies descriptions refer to, by reference key. */
export class DeclarationIndex {
    /** The object types, by reference key. */
    readonly objects = new Map<string, ObjectType>();
    /** The policies, the object types' own included, by reference key. */
    readonly policies = new Map<string, Policy>();

    /** Index known declarations beside the intrinsic policies every access model has. */
    static of(known: Iterable<KnownDeclaration>): DeclarationIndex {
        const declarations = new DeclarationIndex();
        for (const declaration of [...INTRINSIC_POLICIES, ...known]) {
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
        } else {
            this.policies.set(ObjectTypeReference.key(declaration.typeReference), declaration);
        }
    }

    /** Read a declaration by key, refusing one the index lacks. */
    require<Value>(declarations: ReadonlyMap<string, Value>, key: string, noun: string): Value {
        const declaration = declarations.get(key);
        if (declaration === undefined) {
            throw new TypeError(`a description refers to unknown ${noun} ${key}`);
        }

        return declaration;
    }
}

/** Read a decoded definition. */
function readDefinition(decoded: unknown): ObjectDefinition;
/**
 * Read a decoded definition as the definition it encodes, which decoding rebuilds exactly.
 *
 * NOTE #Architecture: the definition boundary in reverse, as defineObject states its type.
 */
function readDefinition(decoded: unknown): unknown {
    schema.looseObject({ name: schema.string(), plural: schema.string() }).parse(decoded);

    return decoded;
}

/** A chain of builder calls as data. */
const BuilderCalls = schema
    .array(
        schema.object({ builder: schema.string().min(1), arguments: schema.array(schema.json()) }),
    )
    .min(1);

import * as accessExports from "@destack/access";
import { Policy, PolicySubject } from "@destack/access";
import { TABLE, type Table } from "@destack/db";
import { DeclarationName, graph, Package } from "@destack/package";
import { NO_STATE, ResourceDeclaration, ResourceKind } from "@destack/resource";
import {
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
import * as objectIndexExports from "./index.ts";
import * as fieldExports from "../field/index.ts";
import * as methodExports from "../method/index.ts";
import * as traitExports from "../trait/index.ts";
import * as replicaExports from "../replica/index.ts";
import * as queryExports from "../query/index.ts";
import * as errorExports from "../error/index.ts";
import * as auditExports from "../audit/index.ts";
import * as policyAccessExports from "../access/index.ts";
import * as historyExports from "../history/index.ts";
import * as sequenceExports from "../sequence/index.ts";
import * as textExports from "../text/index.ts";
import * as stackExports from "../stack/index.ts";
import * as branchExports from "../branch/index.ts";
import * as claimExports from "../claim/index.ts";
import * as hookExports from "../hook/index.ts";
import * as declareExports from "../declare/index.ts";
import * as procedureExports from "../procedure/index.ts";
import * as healthExports from "../health/index.ts";
import * as bodyExports from "../server/body.ts";

/** The definition keys that may have lazy references, `*` for any and `[]` for list entries. */
const LAZY_PATHS: readonly (readonly string[])[] = [
    ["indexes", "*", "across"],
    ["constraints", "*", "references", "object"],
];

/** The builders taking lazy function references, such as `field.reference(() => note)`. */
const LAZY_BUILDERS: ReadonlySet<string> = new Set(["reference"]);

/** The path of a value a package exports: the export's name, then the keys inside it. */
const ExportPath = schema.object({
    /** The exporting package. */
    package: schema.string().min(1),
    /** The export's name, then the keys inside it. */
    path: schema.array(schema.string().min(1)).min(1),
});
/** The path of a value a package exports. */
type ExportPath = schema.Infer<typeof ExportPath>;

/** The exports descriptions refer to by path: object's own and its dependency access's. */
export class ExportIndex {
    /** The path of each exported code value: an export, an object's entry or a type's method. */
    readonly #paths = new Map<unknown, ExportPath>();
    /** The value at each recorded path, by the path's key. */
    readonly #values = new Map<string, unknown>();
    /** The index of the loaded modules, built at the first read. */
    static #loaded: ExportIndex | undefined;

    /** Index the exports of module namespaces. */
    constructor(modules: Readonly<Record<string, Readonly<Record<string, unknown>>>>) {
        // record each export, each entry of an exported plain object and each method of an exported type, the first path kept
        for (const [name, module] of Object.entries(modules)) {
            for (const [key, value] of Object.entries(module)) {
                this.#record(value, { package: name, path: [key] });
                const type: ObjectType | undefined =
                    value instanceof ObjectType ? value : undefined;
                const entries =
                    type !== undefined
                        ? Object.entries(type.methods).map(
                              ([named, entry]) => [["methods", named], entry] as const,
                          )
                        : isRecord(value)
                          ? Object.entries(value).map(([inner, entry]) => [[inner], entry] as const)
                          : [];
                for (const [inner, entry] of entries) {
                    this.#record(entry, { package: name, path: [key, ...inner] });
                }
            }
        }
    }

    /** Index object's own exports, read from the modules its root exports, and access's. */
    static loaded(): ExportIndex {
        // index each module once, at the first call, beside the root whose own evaluation may be running
        ExportIndex.#loaded ??= new ExportIndex({
            "@destack/object": {
                ...objectIndexExports,
                ...fieldExports,
                ...methodExports,
                ...traitExports,
                ...replicaExports,
                ...queryExports,
                ...errorExports,
                ...auditExports,
                ...policyAccessExports,
                ...historyExports,
                ...sequenceExports,
                ...textExports,
                ...stackExports,
                ...branchExports,
                ...claimExports,
                ...hookExports,
                ...declareExports,
                ...procedureExports,
                ...healthExports,
                ...bodyExports,
            },
            "@destack/access": accessExports,
        });

        return ExportIndex.#loaded;
    }

    /** Find the path of an exported value, absent for one no indexed module exports. */
    locate(value: unknown): ExportPath | undefined {
        return this.#paths.get(value);
    }

    /** Read the value at a recorded export path, refusing any other. */
    resolve(path: ExportPath): unknown {
        return present(
            this.#values.get(ExportIndex.#key(path)),
            `the export ${path.package}#${path.path.join(".")}`,
        );
    }

    /** List the exported object types and policies. */
    declarations(): KnownDeclaration[] {
        return [...this.#paths.keys()].filter(
            (value): value is KnownDeclaration =>
                value instanceof ObjectType || value instanceof Policy,
        );
    }

    /** List the exported tables. */
    tables(): Table[] {
        return [...this.#paths.keys()].filter(isTable);
    }

    /** Record a code value's path once, leaving plain data, which descriptions write as it is. */
    #record(value: unknown, path: ExportPath): void {
        const isCode = typeof value === "function" || (typeof value === "object" && value !== null);
        const isData = Array.isArray(value) || (isRecord(value) && !Method.is(value));
        if (isCode && !isData) {
            this.#values.set(ExportIndex.#key(path), value);
            if (!this.#paths.has(value)) {
                this.#paths.set(value, path);
            }
        }
    }

    /** Key an export path by its package and keys. */
    static #key(path: ExportPath): string {
        return JSON.stringify([path.package, ...path.path]);
    }
}

/** The kind of the declarations `defineObject` makes, as manifests list them. */
export const OBJECT_DECLARATION = DeclarationName.parse("object");

/** A builder call as written: the builder's name and its arguments. */
export interface BuilderCall {
    /** The builder: a `field` or `method` function, or a modifier of an earlier result. */
    readonly builder: string;
    /** The arguments, in order. */
    readonly arguments: readonly unknown[];
}

/** A declaration a description may refer to: an object type or a policy. */
export type KnownDeclaration = ObjectType | Policy;

/** An object type as data: its package and its definition as written, its special values tagged. */
const objectDescription = schema.object({
    /** The type's name, which names its declaration. */
    name: schema.string().min(1),
    /** The declaring package. */
    package: Package,
    /** The definition as written beside its name, special values tagged. */
    definition: schema.jsonObject(),
});

/** An object type as data, with the verbs reading a description. */
export const ObjectDescription = Object.assign(objectDescription, {
    /** Encode an object type as data. */
    of(type: ObjectType): ObjectDescription {
        // require a valid type, then tag the definition's special values, keeping its name beside it
        type.requireValid();
        const { source } = type;
        const { name: _name, ...definition } = Position.root(type.name).encodeObject(source);

        return { name: type.name, package: identity(type.package), definition };
    },

    /** Key a description's type by its policy: its own, or the principal kind it represents. */
    key(description: ObjectDescription): string {
        const { represents } = description.definition;
        const reference =
            represents === undefined
                ? { packageId: description.package.id, type: description.name }
                : schema.object({ $policy: ObjectTypeReference }).parse(represents).$policy;

        return ObjectTypeReference.key(reference);
    },

    /** Read how a stack document declares a description's type, absent when stacks do not. */
    declaration(
        description: ObjectDescription,
    ): { readonly plural: string; readonly isNested: boolean } | undefined {
        const { plural, declarable, nested } = DescribedDefinition.parse(description.definition);

        return declarable === undefined ? undefined : { plural, isNested: nested !== undefined };
    },

    /** Read the name of the resource kind a description's type provisions, absent for none. */
    provisionedKind(description: ObjectDescription): string | undefined {
        return DescribedDefinition.parse(description.definition).provisioned?.kind.$kind.name;
    },

    /** Read the object descriptions among an installation's declarations. */
    installed(
        declarations: readonly Pick<graph.Declaration, "kind" | "description">[],
    ): ObjectDescription[] {
        return declarations
            .filter((entry) => entry.kind === OBJECT_DECLARATION)
            .map((entry) => ObjectDescription.parse(entry.description));
    },

    /** Decode a description's definition against the known declarations, its methods built. */
    decode(description: ObjectDescription, known: ObjectTypePool): ObjectDefinition {
        const definition = { ...description.definition, name: description.name };

        return readDefinition(known.decode(definition));
    },
});
/** An object type as data. */
export type ObjectDescription = schema.Infer<typeof objectDescription>;

/** The object types and policies descriptions are declared against, by reference key. */
export class ObjectTypePool {
    /** The object types, by reference key. */
    readonly objects = new Map<string, ObjectType>();
    /** The policies, the object types' own included, by reference key. */
    readonly policies = new Map<string, Policy>();
    /** The exports descriptions refer to by path. */
    readonly exports = ExportIndex.loaded();
    /** The tables object types map, by SQL name: the known types' and the exported ones. */
    readonly tables = new Map<string, Table>(
        this.exports.tables().map((table) => [table[TABLE].sqlName, table]),
    );

    /** Pool known declarations beside the exported ones. */
    static of(known: Iterable<KnownDeclaration>): ObjectTypePool {
        const declarations = new ObjectTypePool();
        for (const declaration of [...declarations.exports.declarations(), ...known]) {
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
            this.tables.set(declaration.table[TABLE].sqlName, declaration.table);
        } else {
            this.policies.set(ObjectTypeReference.key(declaration.typeReference), declaration);
        }
    }

    /** Declare object types from descriptions and add them, each once its references are known. */
    declare(descriptions: readonly ObjectDescription[]): ObjectType[] {
        // declare each type once the object types of the same descriptions it refers to now are
        //  declared, as their tables are the ones its own refer to; policies match by key alone
        const pending = new Map(descriptions.map((each) => [ObjectDescription.key(each), each]));
        const declared = new Map<string, ObjectType>();
        while (pending.size > 0) {
            const ready = [...pending].filter(([key, each]) =>
                ObjectTypePool.#references(each.definition, false).every(({ reference, kind }) => {
                    const referred = ObjectTypeReference.key(reference);

                    return kind === "policies" || referred === key || !pending.has(referred);
                }),
            );
            if (ready.length === 0) {
                throw new TypeError(
                    `object types ${[...pending.keys()].join(", ")} refer to each other now; refer to one lazily`,
                );
            }
            for (const [key, each] of ready) {
                const { methods, ...definition } = ObjectDescription.decode(each, this);
                const type = ObjectType.declare(
                    { ...definition, ...(methods === undefined ? {} : { methods }) },
                    { package: each.package },
                );
                this.add(type);
                declared.set(key, type);
                pending.delete(key);
            }
        }

        return descriptions.map((each) =>
            present(declared.get(ObjectDescription.key(each)), "the declared type"),
        );
    }

    /** List the types descriptions refer to that neither the pool nor the descriptions declare. */
    missing(descriptions: readonly ObjectDescription[]): ObjectTypeReference[] {
        const described = new Set(descriptions.map((each) => ObjectDescription.key(each)));
        const missing = descriptions
            .flatMap((each) => this.references(each))
            .filter((reference) => !described.has(ObjectTypeReference.key(reference)));

        return [...new Map(missing.map((each) => [ObjectTypeReference.key(each), each])).values()];
    }

    /** List the types and policies a description refers to that the pool lacks, lazy ones too. */
    references(description: ObjectDescription): ObjectTypeReference[] {
        const references = ObjectTypePool.#references(description.definition, true)
            .filter(({ reference, kind }) => !this[kind].has(ObjectTypeReference.key(reference)))
            .map(({ reference }) => [ObjectTypeReference.key(reference), reference] as const);

        return [...new Map(references).values()];
    }

    /** Report whether the pool knows a type or policy by its reference. */
    has(reference: ObjectTypeReference): boolean {
        return this.policies.has(ObjectTypeReference.key(reference));
    }

    /** Rebuild a description's values, resolving each tag against the pool. */
    decode(value: JsonValue): unknown {
        // take plain values as they are
        if (value === null || typeof value !== "object") {
            return value;
        }
        // rebuild each entry of a list
        else if (isList(value)) {
            return value.map((entry) => this.decode(entry));
        }

        // resolve a tag, or rebuild a plain object's entries
        const tag = Object.keys(value).find((key) => key.startsWith("$"));

        return tag === undefined
            ? Object.fromEntries(
                  JsonValue.entries(value).map(([key, entry]) => [key, this.decode(entry)]),
              )
            : this.#decodeTag(tag, value[tag]);
    }

    /** Resolve one tagged value. */
    #decodeTag(tag: string, value: JsonValue | undefined): unknown {
        switch (tag) {
            case "$object":
                return this.#require(this.objects, value, "object type");
            case "$policy":
                return this.#require(this.policies, value, "policy");
            case "$table":
                return this.#requireTable(value);
            case "$kind":
                return DescribedKind.decode(value);
            case "$bucket":
                return DescribedBucket.decode(value);
            case "$lazy":
                return () => this.decode(present(value, "the lazy declaration"));
            case "$schema":
                return fromJsonSchema(DescribedSchema.parse(value));
            case "$field":
                return this.#replay(field, FIELD_MODIFIERS, BuilderCalls.parse(value));
            case "$method":
                return this.#replay(method, [], BuilderCalls.parse(value));
            case "$export":
                return this.exports.resolve(ExportPath.parse(value));
            default:
                throw new TypeError(`a description has an unknown tag ${tag}`);
        }
    }

    /** Replay a chain of builder calls: one of the given builders, then modifiers of its result. */
    #replay(
        builders: object,
        modifiers: readonly string[],
        calls: schema.Infer<typeof BuilderCalls>,
    ): unknown {
        // call the builder, then each modifier on what it built
        const [first, ...rest] = calls;
        let built = this.#invoke(builders, present(first, "the first builder call"));
        for (const each of rest) {
            if (!modifiers.includes(each.builder)) {
                throw new TypeError(
                    `a description modifies with ${each.builder}, which is not a modifier`,
                );
            }
            built = this.#invoke(built, each);
        }

        return built;
    }

    /** Call a builder its owner has, with the arguments decoded, refusing any other name. */
    #invoke(owner: unknown, built: schema.Infer<typeof BuilderCalls>[number]): unknown {
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
            built.arguments.map((argument) => this.decode(argument)),
        );
    }

    /** List the type and policy references in a description's values, lazy ones when asked. */
    static #references(
        value: JsonValue,
        isLazyFollowed: boolean,
    ): { readonly reference: ObjectTypeReference; readonly kind: "objects" | "policies" }[] {
        // read nothing from plain values, and each entry of a list
        if (value === null || typeof value !== "object") {
            return [];
        } else if (isList(value)) {
            return value.flatMap((entry) => ObjectTypePool.#references(entry, isLazyFollowed));
        }
        // pass a type or policy reference
        else if ("$object" in value) {
            return [{ reference: ObjectTypeReference.parse(value["$object"]), kind: "objects" }];
        } else if ("$policy" in value) {
            return [{ reference: ObjectTypeReference.parse(value["$policy"]), kind: "policies" }];
        }
        // follow a lazy reference only when asked, and every other entry
        else if ("$lazy" in value && !isLazyFollowed) {
            return [];
        }

        return JsonValue.values(value).flatMap((entry) =>
            ObjectTypePool.#references(entry, isLazyFollowed),
        );
    }

    /** Read a table by its tagged SQL name, refusing one the pool lacks. */
    #requireTable(value: JsonValue | undefined): Table {
        // find the table by its SQL name
        const name = schema.string().parse(value);
        const table = this.tables.get(name);
        if (table === undefined) {
            throw new TypeError(
                `a description maps the table ${name}, which no known type or loaded package maps`,
            );
        }

        return table;
    }

    /** Read a declaration by its tagged reference, refusing one the pool lacks. */
    #require<Value>(
        declarations: ReadonlyMap<string, Value>,
        value: JsonValue | undefined,
        noun: string,
    ): Value {
        // look the reference up, refusing one the pool lacks
        const key = ObjectTypeReference.key(ObjectTypeReference.parse(value));
        const declaration = declarations.get(key);
        if (declaration === undefined) {
            throw new TypeError(`a description refers to unknown ${noun} ${key}`);
        }

        return declaration;
    }
}

/** Where a value sits in a definition being described, which encodes the values found there. */
class Position {
    /** The path messages show, such as `task.indexes.name.across`. */
    readonly path: string;
    /** The keys from the definition's root, absent inside a builder's arguments. */
    readonly keys: readonly string[] | undefined;
    /** Whether the value may be a lazy reference to types or policies declared later. */
    readonly isLazy: boolean;
    /** The exports code values are written as paths of. */
    readonly exports: ExportIndex;

    /** Place a value at a path, under definition keys, lazy or not. */
    constructor(
        path: string,
        keys: readonly string[] | undefined,
        isLazy: boolean,
        exports: ExportIndex,
    ) {
        // keep the path, keys, laziness and exports
        this.path = path;
        this.keys = keys;
        this.isLazy = isLazy;
        this.exports = exports;
    }

    /** Place a definition's root at its type's name. */
    static root(name: string): Position {
        return new Position(name, [], false, ExportIndex.loaded());
    }

    /** Place an entry: its path, definition keys and whether it may be lazy. */
    entry(key: string, path: string): Position {
        const keys = this.keys === undefined ? undefined : [...this.keys, key];
        const isLazy =
            keys !== undefined &&
            LAZY_PATHS.some(
                (pattern) =>
                    pattern.length === keys.length &&
                    pattern.every((each, offset) => each === "*" || each === keys[offset]),
            );

        return new Position(path, keys, isLazy, this.exports);
    }

    /** Tag a plain object's special values, refusing keys that start like a tag. */
    encodeObject(value: object): JsonObject {
        // refuse keys that start like a tag
        const entries = Object.entries(value).filter(([, entry]) => entry !== undefined);
        if (entries.some(([key]) => key.startsWith("$"))) {
            throw new TypeError(
                `${this.path} has a key starting with $: descriptions reserve it for tags`,
            );
        }

        // tag each present entry
        return Object.fromEntries(
            entries.map(([key, entry]) => [
                key,
                this.entry(key, `${this.path}.${key}`).encode(entry),
            ]),
        );
    }

    /** Tag a value's types, exported code, fields, methods, schemas, kinds and buckets. */
    encode(value: unknown): JsonValue {
        // find the value's export path, then take plain data as it is
        const exported = this.exports.locate(value);
        if (
            value === null ||
            typeof value === "string" ||
            typeof value === "number" ||
            typeof value === "boolean"
        ) {
            return value;
        } else if (Array.isArray(value)) {
            return value.map((entry, offset) =>
                this.entry("[]", `${this.path}[${offset}]`).encode(entry),
            );
        }
        // tag types and policies by reference, another package's table by its name, kinds and buckets as data
        else if (isTable(value)) {
            return { $table: value[TABLE].sqlName };
        } else if (value instanceof ObjectType) {
            return { $object: value.typeReference };
        } else if (value instanceof Policy) {
            return { $policy: value.typeReference };
        } else if (value instanceof PolicySubject) {
            return this.encode(value.type());
        } else if (value instanceof ResourceKind) {
            return { $kind: DescribedKind.of(value) };
        } else if (value instanceof ResourceDeclaration) {
            return { $bucket: DescribedBucket.of(value) };
        }
        // write exported code as its export path, keeping its identity
        else if (exported !== undefined) {
            return { $export: exported };
        }
        // tag fields, methods and schemas by how they were built
        else if (value instanceof Field) {
            return { $field: value.describe().map((call) => this.encodeCall(call)) };
        } else if (Method.is(value) && value.source !== undefined) {
            return { $method: [this.encodeCall(value.source)] };
        } else if (value instanceof schema.Schema) {
            return { $schema: JsonValue.object(toJsonSchema(value)) };
        }
        // tag a lazy reference where the definition takes one
        else if (typeof value === "function" && this.isLazy) {
            return this.#encodeLazy(value);
        }

        // tag a plain object's entries, and refuse other code
        if (Method.is(value)) {
            throw new TypeError(
                `${this.path} has a handler; attach it with .implement({ methods })`,
            );
        } else if (typeof value === "object" && Object.getPrototypeOf(value) === Object.prototype) {
            return this.encodeObject(value);
        } else {
            throw new TypeError(
                `${this.path} is code no loaded package exports: a description cannot store it`,
            );
        }
    }

    /** Tag a builder call, its arguments tagged and trailing absent ones left out. */
    encodeCall(call: BuilderCall): JsonObject {
        // leave out trailing absent arguments
        const written = [...call.arguments];
        while (written.length > 0 && written.at(-1) === undefined) {
            written.pop();
        }

        // tag each argument, a lazy reference where the builder takes one
        const isLazy = LAZY_BUILDERS.has(call.builder);

        return {
            builder: call.builder,
            arguments: written.map((argument, offset) =>
                new Position(
                    `${this.path}.${call.builder}[${offset}]`,
                    undefined,
                    isLazy,
                    this.exports,
                ).encode(argument),
            ),
        };
    }

    /** Tag a lazy reference by its returned types or policies, refusing other returns. */
    #encodeLazy(value: Function): JsonValue {
        // resolve the reference, refusing one returning anything but types or policies
        const resolved: unknown = Reflect.apply(value, undefined, []);
        const returned = Array.isArray(resolved) ? resolved : [resolved];
        if (
            returned.length === 0 ||
            !returned.every((each) => each instanceof ObjectType || each instanceof Policy)
        ) {
            throw new TypeError(`${this.path} returns no object types or policies`);
        }

        return { $lazy: new Position(this.path, this.keys, false, this.exports).encode(resolved) };
    }
}

/** A JSON Schema as a description holds it. */
const DescribedSchema = schema.jsonObject();

/** A resource kind as a description holds it: its package, name and schemas. */
const DescribedKind = Object.assign(
    schema.object({
        /** The declaring package. */
        package: Package,
        /** The kind's name. */
        name: DeclarationName,
        /** The schema of a declaration's specification. */
        spec: DescribedSchema,
        /** The schema of the desired state its providers reconcile, null for a kind without one. */
        state: DescribedSchema.nullable(),
    }),
    {
        /** Describe a resource kind: its package, name and JSON Schemas. */
        of(
            kind: Pick<ResourceKind, "package" | "name" | "spec" | "state" | "isStateful">,
        ): JsonObject {
            return {
                package: identity(kind.package),
                name: kind.name,
                spec: JsonValue.object(toJsonSchema(kind.spec)),
                state: kind.isStateful ? JsonValue.object(toJsonSchema(kind.state)) : null,
            };
        },

        /** Rebuild a resource kind a description holds. */
        decode(value: JsonValue | undefined): ResourceKind {
            const { package: owner, name, spec, state } = DescribedKind.parse(value);

            return new ResourceKind(
                owner,
                name,
                fromJsonSchema(spec),
                state === null ? NO_STATE : fromJsonSchema(state),
            );
        },
    },
);

/** A bucket as a description holds it: its package, name and specification. */
const DescribedBucket = Object.assign(
    schema.object({
        /** The declaring package. */
        package: Package,
        /** The bucket's name. */
        name: DeclarationName,
        /** The bucket's specification. */
        spec: schema.jsonObject(),
    }),
    {
        /** Describe a bucket as data: its package, name and specification. */
        of(bucket: {
            readonly package: Package;
            readonly name: string;
            readonly spec: Readonly<Record<string, JsonValue>>;
        }): JsonObject {
            return { package: identity(bucket.package), name: bucket.name, spec: bucket.spec };
        },

        /** Rebuild a bucket a description holds. */
        decode(value: JsonValue | undefined): FileBucket {
            const { package: owner, ...bucket } = DescribedBucket.parse(value);

            return new ResourceDeclaration(owner, { ...bucket, kind: "bucket" });
        },
    },
);

/** A chain of builder calls as data. */
const BuilderCalls = schema
    .array(
        schema.object({ builder: schema.string().min(1), arguments: schema.array(schema.json()) }),
    )
    .min(1);

/** The fields of a described definition that stack documents read: plural, declaration and kind. */
const DescribedDefinition = schema.looseObject({
    /** The type's plural, the document key of its declarations. */
    plural: schema.string(),
    /** How stacks declare the type, absent on a type stacks do not declare. */
    declarable: schema.unknown().exactOptional(),
    /** The type its records nest in, whose declarations contain the type's own. */
    nested: schema.unknown().exactOptional(),
    /** The resource kind the type provisions, absent on a type provisioning none. */
    provisioned: schema
        .looseObject({ kind: schema.looseObject({ $kind: DescribedKind }) })
        .exactOptional(),
});

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

/** Decide whether a JSON value is a list. */
function isList(value: readonly JsonValue[] | JsonObject): value is readonly JsonValue[] {
    return Array.isArray(value);
}

/** Read only a package's identity. */
function identity(owner: Package): Package {
    return { id: owner.id, name: owner.name, version: owner.version };
}

/** Report whether a value is a plain object or a module namespace, which export paths read. */
function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
    const prototype: unknown =
        typeof value === "object" && value !== null ? Object.getPrototypeOf(value) : undefined;

    return prototype === Object.prototype || prototype === null;
}

/** Check that a value is a table, which descriptions name instead of encoding. */
function isTable(value: unknown): value is Table {
    return typeof value === "object" && value !== null && TABLE in value;
}

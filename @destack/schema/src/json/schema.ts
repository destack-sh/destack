import { RE2JS } from "re2js";
import { z } from "zod";
import { HASH_ALGORITHMS, HASH_ENCODINGS, requireDeclarable } from "../validate/declarable.ts";
import { canonicalize, type JsonObject } from "./json.ts";
import { present } from "../presence/presence.ts";

/** The descriptions of the schemas described so far, kept by schema. */
const DESCRIBED = new WeakMap<z.ZodType, JsonSchema>();

/** The JSON Schema Draft 2020-12 description for inspection and external tooling. */
export type JsonSchema = z.core.JSONSchema.JSONSchema;

/** The prefix of a reference to one of the document's own definitions. */
const DEFINITION = "#/$defs/";

/** The bound `toJsonSchema` gives every integer: the largest a double represents exactly. */
const SAFE_INTEGER = Number.MAX_SAFE_INTEGER;

/** The hash algorithms and encodings declared strings may check, each pair a format. */
const HASHES = HASH_ALGORITHMS.flatMap((algorithm) =>
    HASH_ENCODINGS.map(
        (encoding) =>
            [`${algorithm}_${encoding}`, () => z.hash(algorithm, { enc: encoding })] as const,
    ),
);

/** The string builders of each JSON Schema format, told apart by the pattern each describes. */
const FORMAT_BUILDERS: Readonly<Record<string, readonly (() => z.ZodStringFormat)[]>> = {
    uri: [() => z.url()],
    email: [() => z.email()],
    emoji: [() => z.emoji()],
    base64: [() => z.base64()],
    base64url: [() => z.base64url()],
    uuid: [() => z.uuid(), () => z.uuidv4(), () => z.uuidv6(), () => z.uuidv7(), () => z.guid()],
    nanoid: [() => z.nanoid()],
    cuid2: [() => z.cuid2()],
    ulid: [() => z.ulid()],
    xid: [() => z.xid()],
    ksuid: [() => z.ksuid()],
    hostname: [() => z.hostname()],
    hex: [() => z.hex()],
    ipv4: [() => z.ipv4()],
    ipv6: [() => z.ipv6()],
    mac: [() => z.mac()],
    cidrv4: [() => z.cidrv4()],
    cidrv6: [() => z.cidrv6()],
    e164: [() => z.e164()],
    jwt: [() => z.jwt()],
    "date-time": [() => z.iso.datetime()],
    date: [() => z.iso.date()],
    duration: [() => z.iso.duration()],
    credit_card: [() => z.creditCard()],
    currency_code: [() => z.currencyCode()],
    iban: [() => z.iban()],
    ...Object.fromEntries(HASHES.map(([format, create]) => [format, [create]])),
};

/** The builders of each format with the pattern each describes, none for a format without one. */
const FORMATS: ReadonlyMap<string, readonly FormatBuilder[]> = new Map(
    Object.entries(FORMAT_BUILDERS).map(([format, builders]) => [
        format,
        builders.map((create) => ({ create, pattern: z.toJSONSchema(create()).pattern })),
    ]),
);

/** The descriptive keywords any node may have, which the built validator keeps as metadata. */
const ANNOTATIONS = ["title", "description", "deprecated", "examples"] as const;

/** The description of any JSON value, as `toJsonSchema` describes `schema.json()`. */
const JSON_VALUE: Readonly<Record<string, unknown>> = Object.fromEntries(
    Object.entries(z.toJSONSchema(z.json(), { target: "draft-2020-12" })).filter(
        ([key]) => key !== "$schema",
    ),
);

/** The description of any JSON value as text, compared with a node of the same keys. */
const JSON_VALUE_TEXT = JSON.stringify(JSON_VALUE);

/** The keywords a string node may have. */
const STRING_KEYWORDS = [
    "type",
    "format",
    "contentEncoding",
    "pattern",
    "minLength",
    "maxLength",
    "allOf",
];

/** The keywords a number node may have. */
const NUMBER_KEYWORDS = [
    "type",
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "multipleOf",
];

/** The keywords an array node may have. */
const ARRAY_KEYWORDS = ["type", "items", "prefixItems", "minItems", "maxItems", "readOnly"];

/** The keywords an object node may have. */
const OBJECT_KEYWORDS = ["type", "properties", "required", "additionalProperties", "propertyNames"];

/** The keywords any node may have beside its validation keywords. */
const DOCUMENT_KEYWORDS = new Set<string>(["$schema", "$defs", ...ANNOTATIONS]);

/** A reference within one JSON Schema document: the document itself, or one of its definitions. */
export const LocalReference = {
    /** Read the definition a reference points at, null for the document itself. */
    definition(reference: string): string | null {
        if (reference === "#") {
            return null;
        } else if (reference.startsWith(DEFINITION) && reference.length > DEFINITION.length) {
            return reference.slice(DEFINITION.length);
        }

        throw new TypeError(
            `schema reference ${reference} points outside the document's definitions`,
        );
    },
};

/** Describe a schema using JSON Schema Draft 2020-12, as a JSON object. */
export function toJsonSchema(schema: z.ZodType): JsonSchema & JsonObject;
/**
 * Describe a schema using JSON Schema Draft 2020-12.
 *
 * @construct Zod writes JSON Schema as plain JSON values: objects, lists, scalars and null.
 */
export function toJsonSchema(schema: z.ZodType): JsonSchema {
    // reuse the description of a schema described before, as schemas never change
    const kept = DESCRIBED.get(schema);
    if (kept !== undefined) {
        return kept;
    }

    // describe a declarable schema
    requireDeclarable(schema);
    const described = describe(schema);

    // keep it frozen, so a reader changing it fails instead of changing every later reader's
    DESCRIBED.set(schema, frozen(described));

    return described;
}

/** Freeze a JSON value and every value inside it. */
function frozen<Value>(value: Value): Value {
    if (typeof value === "object" && value !== null) {
        for (const inner of Object.values(value)) {
            frozen(inner);
        }
        Object.freeze(value);
    }

    return value;
}

/** Describe a schema as Draft 2020-12, refusing what JSON Schema cannot represent. */
function describe(schema: z.ZodType): JsonSchema {
    return z.toJSONSchema(schema, {
        target: "draft-2020-12",
        unrepresentable: "throw",
        io: "input",
    });
}

/** Build a validator from JSON Schema as `toJsonSchema` describes declarable schemas. */
export function fromJsonSchema(description: JsonSchema): z.ZodType {
    // resolve the document's own reference to the validator built for it
    let root: z.ZodType | undefined;
    const context = { root: description, references: new Map<string, z.ZodType>() };
    context.references.set(
        "#",
        z.lazy(() => present(root, "the document's validator")),
    );

    // build the document
    root = build(description, context);

    return root;
}

/** A JSON Schema node: an object, or true or false as a schema. */
type JsonSchemaValue = z.core.JSONSchema._JSONSchema;

/** The document a node belongs to, and the validator each local reference resolved to. */
interface ImportContext {
    /** The whole document, which local references point into. */
    readonly root: JsonSchema;
    /** The validator of each local reference, by pointer. */
    readonly references: Map<string, z.ZodType>;
}

/** A format's string builder and the pattern it describes. */
interface FormatBuilder {
    /** Build the format's validator. */
    readonly create: () => z.ZodStringFormat;
    /** The pattern the builder describes, absent for a format without one. */
    readonly pattern: string | undefined;
}

/** Build the validator one node describes, keeping its descriptive keywords as metadata. */
function build(node: JsonSchemaValue, context: ImportContext): z.ZodType {
    // refuse a boolean schema, which no declarable schema describes
    if (typeof node === "boolean") {
        throw new TypeError(`json schema ${String(node)} is not declarable`);
    }

    // build the node, then keep its title, description and examples
    const built = validator(node, context);
    const meta = Object.fromEntries(
        ANNOTATIONS.flatMap((key) => (node[key] === undefined ? [] : [[key, node[key]]])),
    );

    return Object.keys(meta).length === 0 ? built : built.meta(meta);
}

/** Build the validator a node's validation keywords describe. */
function validator(node: JsonSchema, context: ImportContext): z.ZodType {
    const { type } = node;

    // build any JSON value
    if (isJsonValue(node)) {
        return z.json();
    }
    // follow a reference
    else if (node.$ref !== undefined) {
        return checked(referenced(node.$ref, context), node, ["$ref"]);
    }
    // build a union, an intersection, no value, or a discriminated union
    else if (node.anyOf !== undefined) {
        return checked(union(node.anyOf, context), node, ["anyOf"]);
    } else if (node.allOf !== undefined && node.type !== "string") {
        return checked(intersection(node.allOf, context), node, ["allOf"]);
    } else if (node.not !== undefined) {
        return checked(never(node.not), node, ["not"]);
    } else if (node.oneOf !== undefined) {
        return checked(discriminated(node.oneOf, context), node, ["oneOf"]);
    }
    // build a literal or an enumeration
    else if (node.const !== undefined && typeof type === "string") {
        return checked(z.literal(primitive(node.const)), node, ["type", "const"]);
    } else if (node.enum !== undefined && typeof type === "string") {
        return checked(enumerated(node.enum), node, ["type", "enum"]);
    }
    // build a value of several types or of one
    else if (Array.isArray(type)) {
        return typeList(type, node, context);
    } else if (typeof type === "string") {
        return typed(type, node, context);
    }
    // refuse a node of no type
    else {
        throw new TypeError(`json schema ${JSON.stringify(node)} has no type`);
    }
}

/** Decide whether a node describes any JSON value, as `toJsonSchema` describes `schema.json()`. */
function isJsonValue(node: JsonSchema): boolean {
    // compare the text only of a node with the same keys
    const { $schema: _schema, ...rest } = node;
    const keys = Object.keys(rest);
    const isSameKeys =
        keys.length === Object.keys(JSON_VALUE).length && keys.every((key) => key in JSON_VALUE);

    return isSameKeys && JSON.stringify(rest) === JSON_VALUE_TEXT;
}

/** Build the validator of one type. */
function typed(type: string, node: JsonSchema, context: ImportContext): z.ZodType {
    switch (type) {
        case "string":
            return checked(string(node), node, STRING_KEYWORDS);
        case "integer":
        case "number":
            return checked(number(type, node), node, NUMBER_KEYWORDS);
        case "boolean":
            return checked(z.boolean(), node, ["type"]);
        case "null":
            return checked(z.null(), node, ["type"]);
        case "array":
            return checked(array(node, context), node, ARRAY_KEYWORDS);
        case "object":
            return checked(object(node, context), node, OBJECT_KEYWORDS);
        default:
            throw new TypeError(`json schema type ${type} is not declarable`);
    }
}

/** Build a value of several types: a constrained type or null, or any of several plain types. */
function typeList(types: readonly string[], node: JsonSchema, context: ImportContext): z.ZodType {
    // build a constrained type followed by null, as a nullable schema describes itself
    const [only, last] = types;
    if (types.length === 2 && only !== undefined && only !== "null" && last === "null") {
        return typed(only, node, context).nullable();
    }

    // build a union of plain types
    const [first, second, ...rest] = types.map((each) => typed(each, {}, context));
    if (first === undefined || second === undefined) {
        throw new TypeError(`json schema types ${types.join(", ")} are not declarable`);
    }

    return checked(z.union([first, second, ...rest]), node, ["type"]);
}

/** Build a string from its format, lengths and patterns, several patterns kept as `allOf`. */
function string(node: JsonSchema): z.ZodType {
    // collect the patterns, one or each of a list
    const { format, pattern, minLength, maxLength, allOf } = node;
    const patterns = [...(pattern === undefined ? [] : [pattern]), ...(allOf ?? []).map(patternOf)];

    // start from the format's builder whose own pattern is among them, or plain text
    const formatted = format === undefined ? undefined : formatOf(format, patterns);
    let built: z.ZodString | z.ZodStringFormat = formatted?.create() ?? z.string();

    // check the lengths and every other pattern
    if (minLength !== undefined) {
        built = built.min(minLength);
    }
    if (maxLength !== undefined) {
        built = built.max(maxLength);
    }
    for (const each of patterns.filter((candidate) => candidate !== formatted?.pattern)) {
        built = built.regex(new LinearPattern(each));
    }

    return built;
}

/** A pattern matched in time linear in its input, as RE2 matches, whatever the pattern. */
class LinearPattern extends RegExp {
    /** The pattern compiled for RE2's linear-time matching. */
    readonly #linear: RE2JS;

    /** Compile a pattern for RE2, refusing the lookarounds and backreferences RE2 lacks. */
    constructor(pattern: string) {
        super(pattern, "u");
        try {
            this.#linear = RE2JS.compile(pattern);
        } catch (error) {
            throw new TypeError(`json schema pattern ${pattern} is not declarable`, {
                cause: error,
            });
        }
    }

    /** Find the pattern in a text, in time linear in the text. */
    override test(text: string): boolean {
        return this.#linear.matcher(text).find();
    }

    /** Refuse matching by position, which only `test` needs. */
    override exec(): never {
        throw new TypeError("a linear pattern supports only test");
    }
}

/** Read the pattern of an `allOf` entry, refusing an entry with anything else. */
function patternOf(entry: JsonSchemaValue): string {
    const keys = typeof entry === "object" ? Object.keys(entry) : [];
    if (typeof entry !== "object" || typeof entry.pattern !== "string" || keys.length !== 1) {
        throw new TypeError("json schema allOf of a string accepts only patterns");
    }

    return entry.pattern;
}

/** Find the builder of a format whose own pattern is among the given ones. */
function formatOf(format: string, patterns: readonly string[]): FormatBuilder {
    const found = FORMATS.get(format)?.find(({ pattern }) =>
        pattern === undefined ? patterns.length === 0 : patterns.includes(pattern),
    );
    if (found !== undefined) {
        return found;
    }

    throw new TypeError(
        `json schema format ${format} with patterns ${patterns.join(", ")} is not declarable`,
    );
}

/** Build a number or integer from its bounds, an integer within the safe range by default. */
function number(type: "integer" | "number", node: JsonSchema): z.ZodType {
    // start from an integer's safe range or any number
    const { minimum, maximum, exclusiveMinimum, exclusiveMaximum } = node;
    let built = type === "integer" ? z.int() : z.number();

    // refuse exclusive bounds as flags
    if (typeof exclusiveMinimum === "boolean" || typeof exclusiveMaximum === "boolean") {
        throw new TypeError("json schema exclusive bounds as flags are not declarable");
    }

    // check the bounds beside an integer's own
    if (minimum !== undefined && !(type === "integer" && minimum === -SAFE_INTEGER)) {
        built = built.min(minimum);
    }
    if (maximum !== undefined && !(type === "integer" && maximum === SAFE_INTEGER)) {
        built = built.max(maximum);
    }
    if (typeof exclusiveMinimum === "number") {
        built = built.gt(exclusiveMinimum);
    }
    if (typeof exclusiveMaximum === "number") {
        built = built.lt(exclusiveMaximum);
    }

    return node.multipleOf === undefined ? built : built.multipleOf(node.multipleOf);
}

/** Build a list from its items, or a tuple from its prefix items. */
function array(node: JsonSchema, context: ImportContext): z.ZodType {
    // read the item keywords
    const { items, prefixItems, minItems, maxItems } = node;
    let built: z.ZodType;

    // build a tuple of its prefix items and any rest, its length bounded as `toJsonSchema` bounds it
    if (prefixItems !== undefined) {
        const rest = items === false ? undefined : items;

        // refuse further items other than one schema, and bounds other than toJsonSchema's
        if (rest !== undefined && (typeof rest !== "object" || Array.isArray(rest))) {
            throw new TypeError("json schema tuples need further items of one schema");
        } else if (
            minItems !== (prefixItems.length === 0 ? undefined : prefixItems.length) ||
            maxItems !== (rest === undefined ? prefixItems.length : undefined)
        ) {
            throw new TypeError("json schema tuples must bound their length by their prefix items");
        }

        // build the prefix items and any rest
        const listed = prefixItems.map((each) => build(each, context));
        built = tuple(listed, rest === undefined ? undefined : build(rest, context));
    }
    // build a list of one item schema, its length bounded
    else if (items !== undefined && typeof items !== "boolean" && !Array.isArray(items)) {
        let list = z.array(build(items, context));
        if (minItems !== undefined) {
            list = list.min(minItems);
        }
        built = maxItems === undefined ? list : list.max(maxItems);
    } else {
        throw new TypeError("json schema arrays without one item schema are not declarable");
    }

    return node.readOnly === true ? built.readonly() : built;
}

/** Build a tuple of some items and any further items of one schema. */
function tuple(listed: readonly z.ZodType[], rest: z.ZodType | undefined): z.ZodType {
    const [first, ...others] = listed;
    if (first === undefined && rest === undefined) {
        return z.tuple([]);
    } else if (first === undefined) {
        throw new TypeError("json schema tuples with further items must start with prefix items");
    }

    return rest === undefined ? z.tuple([first, ...others]) : z.tuple([first, ...others], rest);
}

/** Build an object from its properties, or a record from its property names and values. */
function object(node: JsonSchema, context: ImportContext): z.ZodType {
    // build an object refusing other properties, each property not required optional
    const { properties, required = [], additionalProperties, propertyNames } = node;
    if (propertyNames === undefined && additionalProperties === false) {
        const shape = Object.fromEntries(
            Object.entries(properties ?? {}).map(([key, property]) => {
                const built = build(property, context);

                return [key, required.includes(key) ? built : built.exactOptional()];
            }),
        );

        return z.strictObject(shape);
    }
    // build a record of keys and values
    else if (
        properties === undefined &&
        propertyNames !== undefined &&
        additionalProperties !== undefined &&
        typeof additionalProperties !== "boolean"
    ) {
        const key = recordKey(build(propertyNames, context));
        const keys = key instanceof z.ZodEnum ? key.options.map(String) : undefined;

        // require every key of an enumeration, as a record over it does
        if (node.required !== undefined && canonicalize(node.required) !== canonicalize(keys)) {
            throw new TypeError(
                "json schema records must require exactly the keys of their enumeration",
            );
        }

        // keep an enumeration's keys optional unless it requires them all
        const value = build(additionalProperties, context);

        return keys !== undefined && node.required === undefined
            ? z.partialRecord(key, value)
            : z.record(key, value);
    }

    throw new TypeError(`json schema object ${JSON.stringify(node)} is not declarable`);
}

/** Read a record's key schema, refusing one that does not validate strings. */
function recordKey(built: z.ZodType): z.core.$ZodRecordKey {
    if (isRecordKey(built)) {
        return built;
    }

    throw new TypeError(`json schema property names of type ${built.type} are not declarable`);
}

/** Decide whether a schema is a record key: text, a format, an enumeration, or their union. */
function isRecordKey(built: z.core.$ZodType): built is z.ZodType & z.core.$ZodRecordKey {
    return (
        built instanceof z.ZodString ||
        built instanceof z.ZodStringFormat ||
        built instanceof z.ZodEnum ||
        (built instanceof z.ZodUnion &&
            built.options.every((option: z.core.$ZodType) => isRecordKey(option)))
    );
}

/** Build a union, a nullable value when it lists one schema and null. */
function union(options: readonly JsonSchemaValue[], context: ImportContext): z.ZodType {
    // build a nullable value from a schema followed by null, as a nullable schema describes itself
    const [only, last] = options;
    if (
        only !== undefined &&
        last !== undefined &&
        options.length === 2 &&
        !isNull(only) &&
        isNull(last)
    ) {
        return build(only, context).nullable();
    }

    // build a union of every option
    const [first, second, ...rest] = options.map((option) => build(option, context));
    if (first === undefined || second === undefined) {
        throw new TypeError("json schema unions of fewer than two options are not declarable");
    }

    return z.union([first, second, ...rest]);
}

/** Build the values every schema of a list describes. */
function intersection(schemas: readonly JsonSchemaValue[], context: ImportContext): z.ZodType {
    const [first, ...rest] = schemas.map((each) => build(each, context));
    if (first === undefined) {
        throw new TypeError("json schema allOf without schemas is not declarable");
    }

    return rest.reduce<z.ZodType>((all, each) => z.intersection(all, each), first);
}

/** Build no value, as the negation of any value describes. */
function never(negated: JsonSchemaValue): z.ZodType {
    if (typeof negated !== "object" || Object.keys(negated).length > 0) {
        throw new TypeError("json schema not of a schema other than any value is not declarable");
    }

    return z.never();
}

/** Decide whether a schema describes only null. */
function isNull(node: JsonSchemaValue): boolean {
    return typeof node === "object" && node.type === "null" && Object.keys(node).length === 1;
}

/** Build a union told apart by the one property every option fixes to its own constants. */
function discriminated(options: readonly JsonSchemaValue[], context: ImportContext): z.ZodType {
    // find the property every option fixes
    const objects = options.map((option) => (typeof option === "object" ? option : {}));
    const [head] = objects;
    const key = Object.entries(head?.properties ?? {}).find(([name]) =>
        objects.every((option) => {
            const property = option.properties?.[name];

            return (
                typeof property === "object" &&
                (property.const !== undefined || property.enum !== undefined)
            );
        }),
    )?.[0];
    if (key === undefined) {
        throw new TypeError("json schema oneOf without a property of constants is not declarable");
    }

    // build each option as an object
    const built = options.map((option) => z.instanceof(z.ZodObject).parse(build(option, context)));
    const [first, ...rest] = built;
    if (first === undefined) {
        throw new TypeError("json schema oneOf without options is not declarable");
    }

    return z.discriminatedUnion(key, [first, ...rest]);
}

/** Build one of a fixed set of strings, or of fixed primitives. */
function enumerated(values: readonly unknown[]): z.ZodType {
    // build a string enum, or literals of other primitives
    const literals = values.map(primitive);
    if (literals.every((value) => typeof value === "string")) {
        const [first, ...rest] = literals;
        if (first === undefined) {
            throw new TypeError("json schema enums without values are not declarable");
        }

        return z.enum([first, ...rest]);
    }

    return z.literal(literals);
}

/** Read a literal value, refusing one that is not a primitive. */
function primitive(value: unknown): string | number | boolean | null {
    if (
        value === null ||
        typeof value === "string" ||
        typeof value === "number" ||
        typeof value === "boolean"
    ) {
        return value;
    }

    throw new TypeError(`json schema literal ${JSON.stringify(value)} is not declarable`);
}

/** Resolve a local reference lazily, building its target once, so references may recurse. */
function referenced(reference: string, context: ImportContext): z.ZodType {
    // reuse the validator a reference already resolved to
    const known = context.references.get(reference);
    if (known !== undefined) {
        return known;
    }

    // resolve the target now, and build it when first parsing
    const target = targetOf(context.root, reference);
    let built: z.ZodType | undefined;
    const lazy = z.lazy(() => (built ??= build(target, context)));
    context.references.set(reference, lazy);

    return lazy;
}

/** Read the schema a reference points at: the document itself, or one of its definitions. */
function targetOf(root: JsonSchema, reference: string): JsonSchemaValue {
    // read the document or one of its definitions, refusing one it lacks
    const definition = LocalReference.definition(reference);
    const target = definition === null ? root : root.$defs?.[definition];
    if (target === undefined) {
        throw new TypeError(`schema reference ${reference} does not resolve`);
    }

    return target;
}

/** Refuse a node with keywords its validator does not check. */
function checked(built: z.ZodType, node: JsonSchema, keywords: readonly string[]): z.ZodType {
    const unknown = Object.keys(node).filter(
        (key) => !keywords.includes(key) && !DOCUMENT_KEYWORDS.has(key),
    );
    if (unknown.length > 0) {
        throw new TypeError(`json schema keywords ${unknown.join(", ")} are not declarable`);
    }

    return built;
}

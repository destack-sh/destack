import { z } from "zod";

/** Zod's built-in guard for string and array length checks. */
const LENGTH_CHECK = z.minLength(0)._zod.def.when;

/** The string formats declared schemas may check. */
const STRING_FORMATS = new Set([
    "regex",
    "uuid",
    "guid",
    "nanoid",
    "cuid2",
    "ulid",
    "xid",
    "ksuid",
    "email",
    "url",
    "emoji",
    "hostname",
    "hex",
    "currency_code",
    "jwt",
    "credit_card",
    "iban",
    "ipv4",
    "ipv6",
    "mac",
    "base64",
    "base64url",
    "e164",
    "cidrv4",
    "cidrv6",
    "datetime",
    "date",
    "time",
    "duration",
]);

/** The check kinds declared schemas may export. */
const EXPORTABLE_CHECKS = new Set<string>([
    "less_than",
    "greater_than",
    "multiple_of",
    "min_length",
    "max_length",
    "length_equals",
    "number_format",
    "string_format",
]);

/** The hash formats declared schemas may check, by algorithm and encoding. */
const HASH_FORMAT = /^(?:md5|sha1|sha256|sha384|sha512)_(?:hex|base64|base64url)$/u;

/** Descriptive metadata that cannot replace exported validation rules. */
const METADATA_KEYS = new Set(["id", "title", "description", "deprecated", "examples"]);

/** Require a schema to declare JSON-compatible values with inspectable, non-executable rules. */
export function requireDeclarable(schema: z.core.$ZodType): void {
    requireNode(schema, new Map(), false);
}

/** Require one schema node to be declarable within the visited nodes and optional context. */
function requireNode(
    schema: z.core.$ZodType,
    visited: Map<z.core.$ZodType, Set<boolean>>,
    isOptionalAllowed: boolean,
): void {
    // reject missing values outside optional properties and nonoptional constraints
    if (schema instanceof z.core.$ZodOptional && !isOptionalAllowed) {
        throw new TypeError("optional schemas are only supported as object properties");
    }

    // visit shared and recursive schemas in each optional-value context
    if (!markVisited(visited, schema, isOptionalAllowed)) {
        return;
    }

    // require JSON metadata and exportable rules
    requireMetadata(schema);
    requireRules(schema);

    // visit each component in its optional-value context
    for (const [component, isComponentOptionalAllowed] of componentsOf(schema, isOptionalAllowed)) {
        requireNode(component, visited, isComponentOptionalAllowed);
    }
}

/** Record a schema as visited in an optional-value context. */
function markVisited(
    visited: Map<z.core.$ZodType, Set<boolean>>,
    schema: z.core.$ZodType,
    isOptionalAllowed: boolean,
): boolean {
    // skip a context visited already
    const contexts = visited.get(schema);
    if (contexts?.has(isOptionalAllowed) === true) {
        return false;
    }

    // record this context
    if (contexts) {
        contexts.add(isOptionalAllowed);
    } else {
        visited.set(schema, new Set([isOptionalAllowed]));
    }

    return true;
}

/** Require a schema's metadata to be JSON descriptions that cannot override validation. */
function requireMetadata(schema: z.core.$ZodType): void {
    // keep metadata from overriding validation in the generated description
    const metadata = z.globalRegistry.get(schema);
    for (const key of Object.keys(metadata ?? {})) {
        if (!METADATA_KEYS.has(key)) {
            throw new TypeError(`unsupported schema metadata: ${key}`);
        }
    }

    // require metadata to be JSON
    if (metadata !== undefined) {
        z.json().parse(metadata);
    }
}

/** Require a schema to coerce nothing and to check only by exportable checks. */
function requireRules(schema: z.core.$ZodType): void {
    // reject coercion
    const definition = schema._zod.def;
    if ("coerce" in definition && definition.coerce === true) {
        throw new TypeError("declared schemas cannot coerce values");
    }

    // collect the checks, including a schema that is itself a check
    const checks = [...(definition.checks ?? [])];
    if (schema instanceof z.core.$ZodCheck) {
        checks.push(schema);
    }
    for (const check of checks) {
        requireCheck(check);
    }
}

/** List a schema's components with their optional-value contexts. */
function componentsOf(
    schema: z.core.$ZodType,
    isOptionalAllowed: boolean,
): (readonly [z.core.$ZodType, boolean])[] {
    // accept the leaf schemas
    if (
        schema instanceof z.core.$ZodString ||
        schema instanceof z.core.$ZodNumber ||
        schema instanceof z.core.$ZodBoolean ||
        schema instanceof z.core.$ZodNull ||
        schema instanceof z.core.$ZodEnum ||
        schema instanceof z.core.$ZodNever
    ) {
        return [];
    }
    // require literal values to survive JSON serialization
    else if (schema instanceof z.core.$ZodLiteral) {
        for (const value of schema._zod.def.values) {
            z.json().parse(value);
        }

        return [];
    }
    // inspect schema components before exporting the compiled string pattern
    else if (schema instanceof z.core.$ZodTemplateLiteral) {
        return schema._zod.def.parts.flatMap((part) =>
            typeof part === "object" && part !== null ? [[part, false] as const] : [],
        );
    }
    // require closed objects with declarable properties
    else if (schema instanceof z.core.$ZodObject) {
        if (schema._zod.def.catchall?._zod.def.type !== "never") {
            throw new TypeError("declared object schemas must reject unknown properties");
        }

        return Object.values(schema._zod.def.shape).map((property) => [property, true] as const);
    }
    // visit array elements
    else if (schema instanceof z.core.$ZodArray) {
        return [[schema._zod.def.element, false]];
    }
    // visit tuple items and the rest
    else if (schema instanceof z.core.$ZodTuple) {
        const rest = schema._zod.def.rest;
        const items = rest === null ? schema._zod.def.items : [...schema._zod.def.items, rest];

        return items.map((item) => [item, false] as const);
    }
    // visit record keys and values
    else if (schema instanceof z.core.$ZodRecord) {
        return [
            [schema._zod.def.keyType, false],
            [schema._zod.def.valueType, false],
        ];
    }
    // visit both sides of an intersection
    else if (schema instanceof z.core.$ZodIntersection) {
        return [
            [schema._zod.def.left, isOptionalAllowed],
            [schema._zod.def.right, isOptionalAllowed],
        ];
    }
    // visit each union option
    else if (schema instanceof z.core.$ZodUnion) {
        return schema._zod.def.options.map((option) => [option, isOptionalAllowed] as const);
    }
    // look through nullable and optional wrappers
    else if (schema instanceof z.core.$ZodNullable || schema instanceof z.core.$ZodOptional) {
        return [[schema._zod.def.innerType, isOptionalAllowed]];
    }
    // permit optional branches whose undefined result this schema rejects
    else if (schema instanceof z.core.$ZodNonOptional) {
        return [[schema._zod.def.innerType, true]];
    }
    // visit the schema a lazy one returns
    else if (schema instanceof z.core.$ZodLazy) {
        return [[schema._zod.def.getter(), isOptionalAllowed]];
    }
    // reject every other schema type
    else {
        throw new TypeError(`unsupported schema type: ${schema._zod.def.type}`);
    }
}

/** Require a check to be exportable and free of executable rules. */
function requireCheck(check: z.core.$ZodCheck): void {
    // reject conditional checks other than zod's length guard
    const rule = check._zod.def;
    if (rule.when !== undefined && rule.when !== LENGTH_CHECK) {
        throw new TypeError("declared schemas cannot use conditional checks");
    }

    // allow the exportable check kinds
    if (!EXPORTABLE_CHECKS.has(rule.check)) {
        throw new TypeError(`unsupported schema check: ${rule.check}`);
    }

    // reject stateful regular expression flags
    if (
        "pattern" in rule &&
        rule.pattern instanceof RegExp &&
        (rule.pattern.global || rule.pattern.sticky)
    ) {
        throw new TypeError("declared regular expressions cannot use global or sticky flags");
    }

    // allow the supported string and hash formats
    if (check instanceof z.core.$ZodCheckStringFormat) {
        const format = check._zod.def.format;
        if (!STRING_FORMATS.has(format) && !HASH_FORMAT.test(format)) {
            throw new TypeError(`unsupported string format: ${format}`);
        }
    }

    // reject URL normalization and custom functions
    if ("normalize" in rule && rule.normalize === true) {
        throw new TypeError("declared schemas cannot request URL normalization");
    }
    if ("fn" in rule && !(rule.check === "string_format" && "pattern" in rule)) {
        throw new TypeError("declared schemas cannot use custom validation functions");
    }
}

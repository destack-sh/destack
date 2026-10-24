import { z } from "zod";
import { canonicalize } from "../json/json.ts";

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

/** The hash algorithms declared strings may check. */
export const HASH_ALGORITHMS = ["md5", "sha1", "sha256", "sha384", "sha512"] as const;

/** The encodings of a hash declared strings may check, each with an algorithm a format. */
export const HASH_ENCODINGS = ["hex", "base64", "base64url"] as const;

/** The hash formats declared schemas may check, each an algorithm and an encoding. */
const HASH_FORMATS: ReadonlySet<string> = new Set(
    HASH_ALGORITHMS.flatMap((algorithm) =>
        HASH_ENCODINGS.map((encoding) => `${algorithm}_${encoding}`),
    ),
);

/** Where a schema sits: an object property's outermost schema, inside one, or any other value. */
type Position = "property" | "branch" | "value";

/** Descriptive metadata that cannot replace exported validation rules. */
const METADATA_KEYS = new Set(["id", "title", "description", "deprecated", "examples"]);

/** Require a schema to declare JSON-compatible values with inspectable, non-executable rules. */
export function requireDeclarable(schema: z.core.$ZodType): void {
    requireNode(schema, new Map(), "value");
}

/** Require one schema node to be declarable within the visited nodes at its position. */
function requireNode(
    schema: z.core.$ZodType,
    visited: Map<z.core.$ZodType, Set<Position>>,
    position: Position,
): void {
    // reject optionals outside object properties
    if (schema instanceof z.core.$ZodOptional && position === "value") {
        throw new TypeError("optional schemas are only supported as object properties");
    }
    // reject defaults below an object property's outermost schema
    else if (schema instanceof z.core.$ZodDefault && position !== "property") {
        throw new TypeError("default schemas are only supported as object properties");
    }

    // visit shared and recursive schemas at each position
    if (!markVisited(visited, schema, position)) {
        return;
    }

    // require JSON metadata and exportable rules
    requireMetadata(schema);
    requireRules(schema);

    // visit each component at its position
    for (const [component, inner] of componentsOf(schema, position)) {
        requireNode(component, visited, inner);
    }
}

/** Record a schema as visited at a position. */
function markVisited(
    visited: Map<z.core.$ZodType, Set<Position>>,
    schema: z.core.$ZodType,
    position: Position,
): boolean {
    // skip a position visited already
    const positions = visited.get(schema);
    if (positions?.has(position) === true) {
        return false;
    }

    // record this position
    if (positions) {
        positions.add(position);
    } else {
        visited.set(schema, new Set([position]));
    }

    return true;
}

/** Require a schema's metadata to be JSON descriptions that cannot override validation. */
function requireMetadata(schema: z.core.$ZodType): void {
    // refuse metadata that would override validation in the generated description
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

/** List a schema's components with their positions. */
function componentsOf(
    schema: z.core.$ZodType,
    position: Position,
): (readonly [z.core.$ZodType, Position])[] {
    // keep optionals inside a property's wrappers, and defaults at its outermost schema alone
    const inside = position === "value" ? "value" : "branch";

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
            typeof part === "object" && part !== null ? [[part, "value"] as const] : [],
        );
    }
    // require closed objects with declarable properties
    else if (schema instanceof z.core.$ZodObject) {
        if (schema._zod.def.catchall?._zod.def.type !== "never") {
            throw new TypeError("declared object schemas must reject unknown properties");
        }

        return Object.values(schema._zod.def.shape).map(
            (property) => [property, "property"] as const,
        );
    }
    // visit array elements
    else if (schema instanceof z.core.$ZodArray) {
        return [[schema._zod.def.element, "value"]];
    }
    // visit tuple items and the rest
    else if (schema instanceof z.core.$ZodTuple) {
        const rest = schema._zod.def.rest;
        const items = rest === null ? schema._zod.def.items : [...schema._zod.def.items, rest];

        return items.map((item) => [item, "value"] as const);
    }
    // visit record keys and values
    else if (schema instanceof z.core.$ZodRecord) {
        return [
            [schema._zod.def.keyType, "value"],
            [schema._zod.def.valueType, "value"],
        ];
    }
    // visit both sides of an intersection
    else if (schema instanceof z.core.$ZodIntersection) {
        return [
            [schema._zod.def.left, inside],
            [schema._zod.def.right, inside],
        ];
    }
    // visit each union option
    else if (schema instanceof z.core.$ZodUnion) {
        return schema._zod.def.options.map((option) => [option, inside] as const);
    }
    // look through an optional wrapper at its position
    else if (schema instanceof z.core.$ZodOptional) {
        return [[schema._zod.def.innerType, position]];
    }
    // look through nullable and readonly wrappers
    else if (schema instanceof z.core.$ZodNullable || schema instanceof z.core.$ZodReadonly) {
        return [[schema._zod.def.innerType, inside]];
    }
    // permit optional branches whose undefined result this schema rejects
    else if (schema instanceof z.core.$ZodNonOptional) {
        return [[schema._zod.def.innerType, "branch"]];
    }
    // require a default to be JSON
    else if (schema instanceof z.core.$ZodDefault) {
        const { innerType } = schema._zod.def;
        const first: unknown = schema._zod.def.defaultValue;
        const second: unknown = schema._zod.def.defaultValue;
        if (!z.json().safeParse(first).success) {
            throw new TypeError("declared defaults must be JSON values");
        }
        // refuse a default a function computes
        else if (canonicalize(first) !== canonicalize(second)) {
            throw new TypeError("declared defaults must be constant JSON values");
        }
        // refuse a default its own schema refuses
        else if (!z.safeParse(innerType, first).success) {
            throw new TypeError("declared defaults must match their schema");
        }

        return [[innerType, "value"]];
    }
    // visit the schema a lazy one returns
    else if (schema instanceof z.core.$ZodLazy) {
        return [[schema._zod.def.getter(), position]];
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
        if (!STRING_FORMATS.has(format) && !HASH_FORMATS.has(format)) {
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

import { z } from "zod";
import type { $ZodType, $ZodTypes } from "zod/v4/core";

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

/** The hash formats declared schemas may check, by algorithm and encoding. */
const HASH_FORMAT = /^(?:md5|sha1|sha256|sha384|sha512)_(?:hex|base64|base64url)$/;

/** Descriptive metadata that cannot replace exported validation rules. */
const METADATA_KEYS = new Set(["id", "title", "description", "deprecated", "examples"]);

/** Require a schema to declare JSON-compatible values with inspectable, non-executable rules. */
export function requireDeclarable(schema: $ZodType): void {
    requireNode(schema, new Map(), false);
}

/** Require one schema node to be declarable within the visited nodes and optional context. */
function requireNode(
    schema: $ZodType,
    visited: Map<$ZodType, Set<boolean>>,
    isOptionalAllowed: boolean,
): void {
    // read the zod definition
    const definition = (schema as $ZodTypes)._zod.def;

    // reject missing values outside optional properties and nonoptional constraints
    if (definition.type === "optional" && !isOptionalAllowed) {
        throw new TypeError("optional schemas are only supported as object properties");
    }

    // visit shared and recursive schemas in each optional-value context
    const contexts = visited.get(schema);
    if (contexts?.has(isOptionalAllowed)) {
        return;
    }

    // record this context
    if (contexts) {
        contexts.add(isOptionalAllowed);
    } else {
        visited.set(schema, new Set([isOptionalAllowed]));
    }

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

    // reject coercion
    if ("coerce" in definition && definition.coerce) {
        throw new TypeError("declared schemas cannot coerce values");
    }

    // collect the checks, including a schema that is itself a check
    const checks = [...(definition.checks ?? [])];
    if ("check" in definition) {
        checks.push(schema as unknown as z.core.$ZodCheck);
    }

    // reject executable and unexportable checks
    for (const check of checks) {
        // reject conditional checks other than zod's length guard
        const rule = check._zod.def;
        if (rule.when !== undefined && rule.when !== LENGTH_CHECK) {
            throw new TypeError("declared schemas cannot use conditional checks");
        }

        // allow the exportable check kinds
        switch (rule.check) {
            case "less_than":
            case "greater_than":
            case "multiple_of":
            case "min_length":
            case "max_length":
            case "length_equals":
            case "number_format":
            case "string_format":
                break;
            default:
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
        if (rule.check === "string_format") {
            const format = (rule as z.core.$ZodCheckStringFormatDef).format;
            if (!STRING_FORMATS.has(format) && !HASH_FORMAT.test(format)) {
                throw new TypeError(`unsupported string format: ${format}`);
            }
        }

        // reject URL normalization and custom functions
        if ("normalize" in rule && rule.normalize) {
            throw new TypeError("declared schemas cannot request URL normalization");
        }
        if ("fn" in rule && !(rule.check === "string_format" && "pattern" in rule)) {
            throw new TypeError("declared schemas cannot use custom validation functions");
        }
    }

    // traverse the supported schema definitions
    switch (definition.type) {
        case "string":
        case "number":
        case "boolean":
        case "null":
        case "enum":
        case "never":
            break;
        case "literal":
            // require literal values to survive JSON serialization
            for (const value of definition.values) {
                z.json().parse(value);
            }
            break;
        case "template_literal":
            // inspect schema components before exporting the compiled string pattern
            for (const part of definition.parts) {
                if (typeof part === "object" && part !== null) {
                    requireNode(part, visited, false);
                }
            }
            break;
        case "object":
            if (definition.catchall?._zod.def.type !== "never") {
                throw new TypeError("declared object schemas must reject unknown properties");
            }
            for (const property of Object.values(definition.shape)) {
                requireNode(property, visited, true);
            }
            break;
        case "array":
            requireNode(definition.element, visited, false);
            break;
        case "tuple":
            for (const item of definition.items) {
                requireNode(item, visited, false);
            }
            if (definition.rest) {
                requireNode(definition.rest, visited, false);
            }
            break;
        case "record":
            requireNode(definition.keyType, visited, false);
            requireNode(definition.valueType, visited, false);
            break;
        case "intersection":
            requireNode(definition.left, visited, isOptionalAllowed);
            requireNode(definition.right, visited, isOptionalAllowed);
            break;
        case "union":
            for (const option of definition.options) {
                requireNode(option, visited, isOptionalAllowed);
            }
            break;
        case "nullable":
            requireNode(definition.innerType, visited, isOptionalAllowed);
            break;
        case "optional":
            requireNode(definition.innerType, visited, isOptionalAllowed);
            break;
        case "nonoptional":
            // permit optional branches whose undefined result this schema rejects
            requireNode(definition.innerType, visited, true);
            break;
        case "lazy":
            requireNode(definition.getter(), visited, isOptionalAllowed);
            break;
        default:
            throw new TypeError(`unsupported schema type: ${definition.type}`);
    }
}

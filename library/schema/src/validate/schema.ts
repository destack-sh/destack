import { z } from "zod";
import type { $ZodType, $ZodTypes } from "zod/v4/core";

/** Zod's built-in guard for string and array length checks. */
const LENGTH_CHECK = z.minLength(0)._zod.def.when;

/** Descriptive metadata that cannot replace exported validation rules. */
const METADATA_KEYS = new Set([
    "id",
    "title",
    "description",
    "deprecated",
    "examples",
]);

/** Require JSON validation rules that can be described without executable callbacks. */
export function validate(
    schema: $ZodType,
    visited: Map<$ZodType, Set<boolean>>,
    isProperty: boolean,
): void {
    const definition = (schema as $ZodTypes)._zod.def;

    // permit missing values only in object properties
    if (definition.type === "optional" && !isProperty) {
        throw new TypeError(
            "Optional schemas are only supported as object properties.",
        );
    }

    // visit shared and recursive schemas in each property context
    const contexts = visited.get(schema);
    if (contexts?.has(isProperty)) return;
    if (contexts) contexts.add(isProperty);
    else visited.set(schema, new Set([isProperty]));

    // keep metadata from overriding validation in the generated description
    const metadata = z.globalRegistry.get(schema);
    for (const key of Object.keys(metadata ?? {})) {
        if (!METADATA_KEYS.has(key)) {
            throw new TypeError(
                `Unsupported portable schema metadata: ${key}.`,
            );
        }
    }
    if (metadata !== undefined) z.json().parse(metadata);

    // reject coercion and executable checks before exporting the schema
    if ("coerce" in definition && definition.coerce) {
        throw new TypeError("Portable schemas cannot coerce values.");
    }
    const checks = [...(definition.checks ?? [])];
    if ("check" in definition) {
        checks.push(schema as unknown as z.core.$ZodCheck);
    }
    for (const check of checks) {
        const rule = check._zod.def;
        if (rule.when !== undefined && rule.when !== LENGTH_CHECK) {
            throw new TypeError(
                "Portable schemas cannot use conditional checks.",
            );
        }
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
                throw new TypeError(
                    `Unsupported portable schema check: ${rule.check}.`,
                );
        }
        if (
            "pattern" in rule &&
            rule.pattern instanceof RegExp &&
            rule.pattern.flags !== ""
        ) {
            throw new TypeError(
                "Portable regular expressions cannot use flags.",
            );
        }
        if (rule.check === "string_format") {
            const format = (rule as z.core.$ZodCheckStringFormatDef).format;
            if (
                ![
                    "regex",
                    "uuid",
                    "email",
                    "datetime",
                    "date",
                    "time",
                    "duration",
                ].includes(format)
            ) {
                throw new TypeError(
                    `Unsupported portable string format: ${format}.`,
                );
            }
        }
        if ("fn" in rule) {
            throw new TypeError(
                "Portable schemas cannot use custom validation functions.",
            );
        }
    }

    // traverse the supported schema definitions
    switch (definition.type) {
        case "string":
        case "number":
        case "boolean":
        case "null":
        case "literal":
        case "enum":
        case "never":
            break;
        case "object":
            if (definition.catchall?._zod.def.type !== "never") {
                throw new TypeError(
                    "Portable object schemas must reject unknown properties.",
                );
            }
            for (const property of Object.values(definition.shape)) {
                validate(property, visited, true);
            }
            break;
        case "array":
            validate(definition.element, visited, false);
            break;
        case "tuple":
            for (const item of definition.items) {
                validate(item, visited, false);
            }
            if (definition.rest) validate(definition.rest, visited, false);
            break;
        case "record":
            validate(definition.keyType, visited, false);
            validate(definition.valueType, visited, false);
            break;
        case "intersection":
            validate(definition.left, visited, isProperty);
            validate(definition.right, visited, isProperty);
            break;
        case "union":
            for (const option of definition.options) {
                validate(option, visited, isProperty);
            }
            break;
        case "nullable":
            validate(definition.innerType, visited, isProperty);
            break;
        case "optional":
            validate(definition.innerType, visited, isProperty);
            break;
        case "lazy":
            validate(definition.getter(), visited, isProperty);
            break;
        default:
            throw new TypeError(
                `Unsupported portable schema type: ${definition.type}.`,
            );
    }
}

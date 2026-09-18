import { defineSchema, schema } from "@destack/schema";

/** An OpenTelemetry value represented with explicit JSON types. */
export const ValueDescription = schema.union([
    schema.object({ stringValue: schema.string() }),
    schema.object({ boolValue: schema.boolean() }),
    schema.object({ intValue: schema.string().regex(/^-?\d+$/) }),
    schema.object({
        doubleValue: schema.union([schema.number(), schema.enum(["NaN", "Infinity", "-Infinity"])]),
    }),
    schema.object({ bytesValue: schema.array(schema.number().int().min(0).max(255)) }),
    schema.object({
        get arrayValue() {
            return schema.array(ValueDescription);
        },
    }),
    schema.object({
        get objectValue() {
            return schema.record(schema.string(), ValueDescription);
        },
    }),
    schema.object({}),
]);

/** A JSON representation of an instrumentation value. */
export type ValueDescription = schema.Infer<typeof ValueDescription>;

/** Describe a value without losing binary or non-finite numeric values. */
export function describeValue(value: unknown, ancestors = new Set<object>()): ValueDescription {
    if (value === null || value === undefined) return {};
    if (typeof value === "string") return { stringValue: value };
    if (typeof value === "boolean") return { boolValue: value };
    if (typeof value === "bigint") return { intValue: value.toString() };
    if (typeof value === "number") {
        return {
            doubleValue: Number.isFinite(value)
                ? value
                : value.toString() as "NaN" | "Infinity" | "-Infinity",
        };
    }
    if (value instanceof Uint8Array) return { bytesValue: Array.from(value) };
    if (typeof value !== "object") throw new TypeError("Unsupported telemetry value.");
    if (ancestors.has(value)) throw new TypeError("Cyclic telemetry value.");

    // track ancestors while preserving repeated references in separate branches
    ancestors.add(value);
    try {
        if (Array.isArray(value)) {
            return { arrayValue: value.map((item) => describeValue(item, ancestors)) };
        }
        if (
            Object.getPrototypeOf(value) !== Object.prototype &&
            Object.getPrototypeOf(value) !== null
        ) {
            throw new TypeError("Telemetry objects must contain plain properties.");
        }

        return {
            objectValue: Object.fromEntries(
                Object.entries(value).map(([key, item]) => [key, describeValue(item, ancestors)]),
            ),
        };
    } finally {
        ancestors.delete(value);
    }
}

/** Instrumentation attributes encoded as JSON values. */
export const AttributeDescription = defineSchema(schema.record(schema.string(), ValueDescription));

/** Describe each defined instrumentation attribute. */
export function describeAttributes(attributes: Record<string, unknown>) {
    return Object.fromEntries(
        Object.entries(attributes)
            .filter(([, value]) => value !== undefined)
            .map(([key, value]) => [key, describeValue(value)]),
    );
}

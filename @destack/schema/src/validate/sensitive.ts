import { z } from "zod";

/** The schemas whose values are sensitive, which nothing derived from a request or record keeps, as zod metadata. */
const SENSITIVE = z.registry<{ readonly sensitive: true }>();

/** Mark a schema's values sensitive, so that nothing derived from a request or record keeps them. */
export function sensitive<Value extends z.ZodType>(value: Value): Value {
    SENSITIVE.add(value, { sensitive: true });

    return value;
}

/** Report whether a schema marks its values sensitive. */
export function isSensitive(value: z.ZodType): boolean {
    return SENSITIVE.has(value);
}

/** Leave out of a value what its schema marks sensitive, through objects, arrays and optional, nullable and defaulted values. */
export function redact(definition: z.ZodType, value: unknown): unknown {
    // drop a sensitive value, and keep an absent one
    if (isSensitive(definition)) {
        return undefined;
    } else if (value === undefined || value === null) {
        return value;
    }

    // redact each field of an object
    const wrapped = definition as { unwrap?: () => z.ZodType; shape?: unknown; element?: unknown };
    if (wrapped.shape !== undefined && typeof value === "object" && !Array.isArray(value)) {
        const shape = wrapped.shape as Readonly<Record<string, z.ZodType>>;

        return Object.fromEntries(
            Object.entries(value as Record<string, unknown>).flatMap(([name, field]) => {
                const kept = shape[name] === undefined ? field : redact(shape[name], field);

                return kept === undefined ? [] : [[name, kept]];
            }),
        );
    }
    // redact each element of an array
    else if (wrapped.element !== undefined && Array.isArray(value)) {
        return value.map((entry) => redact(wrapped.element as z.ZodType, entry) ?? null);
    }
    // look through optional, nullable and defaulted wrappers to the schema they wrap
    else if (typeof wrapped.unwrap === "function") {
        return redact(wrapped.unwrap(), value);
    }

    return value;
}

import { z } from "zod";

/** The schemas whose values are sensitive, which nothing derived from a request or record keeps, as zod metadata. */
const SENSITIVE = z.registry<{ readonly sensitive: true }>();

/** Mark a schema's values sensitive, so that nothing derived from a request or record keeps them. */
export function sensitive<Value extends z.ZodType>(value: Value): Value {
    SENSITIVE.add(value, { sensitive: true });

    return value;
}

/** Report whether a schema marks its values sensitive. */
export function isSensitive(value: z.core.$ZodType): boolean {
    return SENSITIVE.has(value);
}

/** Leave out what a schema marks sensitive, handing each dropped value to `found`. */
export function redact(
    definition: z.core.$ZodType,
    value: unknown,
    found?: (sensitive: unknown) => void,
): unknown {
    // drop a sensitive value, and keep an absent one
    if (isSensitive(definition)) {
        if (value !== undefined) {
            found?.(value);
        }

        return undefined;
    } else if (value === undefined || value === null) {
        return value;
    }

    // redact each field of an object
    if (definition instanceof z.ZodObject && typeof value === "object" && !Array.isArray(value)) {
        return redactFields(definition, value, found);
    }
    // redact each element of an array
    else if (definition instanceof z.ZodArray && Array.isArray(value)) {
        return value.map((entry: unknown) => redact(definition.element, entry, found) ?? null);
    }
    // look through optional, nullable, defaulted and read-only wrappers to the schema they wrap
    else if (
        definition instanceof z.core.$ZodOptional ||
        definition instanceof z.core.$ZodNullable ||
        definition instanceof z.core.$ZodDefault ||
        definition instanceof z.core.$ZodPrefault ||
        definition instanceof z.core.$ZodNonOptional ||
        definition instanceof z.core.$ZodReadonly
    ) {
        return redact(definition._zod.def.innerType, value, found);
    }
    // look through a lazy schema to the one it returns
    else if (definition instanceof z.core.$ZodLazy) {
        return redact(definition._zod.def.getter(), value, found);
    }

    return value;
}

/** Leave out the fields of an object that its schema marks sensitive, handing each dropped value to `found`. */
export function redactFields(
    definition: z.ZodObject,
    value: object,
    found?: (sensitive: unknown) => void,
): Record<string, unknown> {
    const shape: Readonly<Record<string, z.core.$ZodType | undefined>> = definition.shape;
    const fields: [string, unknown][] = Object.entries(value);

    return Object.fromEntries(
        fields.flatMap(([name, field]) => {
            const property = shape[name];
            const kept = property === undefined ? field : redact(property, field, found);

            return kept === undefined ? [] : [[name, kept]];
        }),
    );
}

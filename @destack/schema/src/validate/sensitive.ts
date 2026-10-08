import { z } from "zod";
import { JsonValue, type JsonObject } from "../json/json.ts";

/** How a value is sensitive: a secret nothing derived keeps, or personal data kept only sealed under its person's key and erasable. */
export type Sensitivity = "secret" | "personal";

/** The schemas whose values are sensitive and how, as zod metadata. */
const SENSITIVE = z.registry<{ readonly sensitivity: Sensitivity }>();

/** Copy a schema with its values marked sensitive: a secret by default, or personal data. */
export function sensitive<Value extends z.ZodType>(
    value: Value,
    sensitivity: Sensitivity = "secret",
): Value {
    // mark a copy, so other uses of the shared schema stay plain
    const marked = value.clone();
    SENSITIVE.add(marked, { sensitivity });

    return marked;
}

/** Read how a schema marks its values sensitive, absent for values that are not. */
export function sensitivityOf(value: z.core.$ZodType): Sensitivity | undefined {
    return SENSITIVE.get(value)?.sensitivity;
}

/** Report whether a schema marks its values sensitive, secret or personal. */
export function isSensitive(value: z.core.$ZodType): boolean {
    return sensitivityOf(value) !== undefined;
}

/** Leave out what a schema marks sensitive, handing each dropped value to `found`. */
export function redact(
    definition: z.core.$ZodType,
    value: JsonValue | undefined,
    found?: (sensitive: unknown) => void,
): JsonValue | undefined {
    return mapSensitive(definition, value, (held) => {
        found?.(held);

        return undefined;
    });
}

/** Replace each value a schema marks sensitive by what `map` makes of it, leaving out a value it maps to nothing. */
export function mapSensitive(
    definition: z.core.$ZodType,
    value: JsonValue | undefined,
    map: (value: JsonValue, sensitivity: Sensitivity) => JsonValue | undefined,
): JsonValue | undefined {
    // map a sensitive value, and keep an absent one
    const sensitivity = sensitivityOf(definition);
    if (value === undefined) {
        return value;
    } else if (sensitivity !== undefined) {
        return map(value, sensitivity);
    } else if (value === null) {
        return value;
    }

    // map each field of an object
    if (definition instanceof z.ZodObject && JsonValue.isObject(value)) {
        return mapFields(definition, value, map);
    }
    // map each element of an array
    else if (definition instanceof z.ZodArray && JsonValue.isArray(value)) {
        return value.map((entry) => mapSensitive(definition.element, entry, map) ?? null);
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
        return mapSensitive(definition._zod.def.innerType, value, map);
    }
    // look through a lazy schema to the one it returns
    else if (definition instanceof z.core.$ZodLazy) {
        return mapSensitive(definition._zod.def.getter(), value, map);
    }

    return value;
}

/** Leave out the fields of an object that its schema marks sensitive, handing each dropped value to `found`. */
export function redactFields(
    definition: z.ZodObject,
    value: JsonObject,
    found?: (sensitive: unknown) => void,
): JsonObject {
    return mapFields(definition, value, (held) => {
        found?.(held);

        return undefined;
    });
}

/** Replace the sensitive values in the fields of an object by what `map` makes of them. */
function mapFields(
    definition: z.ZodObject,
    value: JsonObject,
    map: (value: JsonValue, sensitivity: Sensitivity) => JsonValue | undefined,
): JsonObject {
    const shape: Readonly<Record<string, z.core.$ZodType | undefined>> = definition.shape;

    return Object.fromEntries(
        Object.entries(value).flatMap(([name, field]) => {
            const property = shape[name];
            const kept = property === undefined ? field : mapSensitive(property, field, map);

            return kept === undefined ? [] : [[name, kept]];
        }),
    );
}

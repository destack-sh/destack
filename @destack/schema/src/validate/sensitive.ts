import { z } from "zod";
import { JsonValue, type JsonObject } from "../json/json.ts";

/** How a value is sensitive: a secret, or personal data. */
export type Sensitivity = "secret" | "personal";

/** The text a masked personal value shows. */
const MASK = "****";

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
    return mapSensitive(definition, value, (marked) => {
        found?.(marked);

        return undefined;
    });
}

/** Report whether a schema marks any of its values personal, at any depth. */
export function marksPersonal(definition: z.core.$ZodType): boolean {
    return marksPersonalIn(definition, new Set());
}

/** Report whether a schema marks a personal value, walking each schema once. */
function marksPersonalIn(definition: z.core.$ZodType, seen: Set<z.core.$ZodType>): boolean {
    // stop at a schema already walked, as recursive lazy schemas return
    if (seen.has(definition)) {
        return false;
    }
    seen.add(definition);

    // read the schema's own mark, then the schemas it contains
    return (
        sensitivityOf(definition) === "personal" ||
        contained(definition).some((child) => marksPersonalIn(child, seen))
    );
}

/** List the schemas a schema contains, through wrappers and lazy schemas. */
function contained(definition: z.core.$ZodType): readonly z.core.$ZodType[] {
    // list an object's fields, an array's element and a tuple's items
    if (definition instanceof z.core.$ZodObject) {
        return Object.values<z.core.$ZodType>(definition._zod.def.shape);
    } else if (definition instanceof z.core.$ZodArray) {
        return [definition._zod.def.element];
    } else if (definition instanceof z.core.$ZodTuple) {
        const { items, rest } = definition._zod.def;

        return rest === null ? items : [...items, rest];
    }
    // list a record's values, a union's options and an intersection's sides
    else if (definition instanceof z.core.$ZodRecord) {
        return [definition._zod.def.valueType];
    } else if (definition instanceof z.core.$ZodUnion) {
        return definition._zod.def.options;
    } else if (definition instanceof z.core.$ZodIntersection) {
        return [definition._zod.def.left, definition._zod.def.right];
    }
    // look through wrappers and lazy schemas
    else if (isWrapper(definition)) {
        return [definition._zod.def.innerType];
    } else if (definition instanceof z.core.$ZodLazy) {
        return [definition._zod.def.getter()];
    }

    return [];
}

/** Mask each personal value a schema marks, leaving every other value as it is. */
export function mask(definition: z.core.$ZodType, value: JsonValue): JsonValue {
    // mask each personal value, which leaves every value present
    const masked = mapSensitive(definition, value, (marked, sensitivity) =>
        sensitivity === "personal" ? maskOf(marked) : marked,
    );
    if (masked === undefined) {
        throw new TypeError("masking skipped a value");
    }

    return masked;
}

/** Replace each sensitive value by what `map` makes of it, leaving out values mapped to nothing. */
export function mapSensitive(
    definition: z.core.$ZodType,
    value: JsonValue | undefined,
    map: (value: JsonValue, sensitivity: Sensitivity) => JsonValue | undefined,
): JsonValue | undefined {
    // map a sensitive value, and answer an absent one as absent
    const sensitivity = sensitivityOf(definition);
    if (value === undefined) {
        return value;
    } else if (sensitivity !== undefined) {
        return map(value, sensitivity);
    } else if (value === null) {
        return value;
    }

    // map each field of an object
    if (definition instanceof z.core.$ZodObject && JsonValue.isObject(value)) {
        return mapFields(definition, value, map);
    }
    // map each element of an array, and each item of a tuple
    else if (definition instanceof z.core.$ZodArray && JsonValue.isArray(value)) {
        const element = definition._zod.def.element;

        return value.map((entry) => mapSensitive(element, entry, map) ?? null);
    } else if (definition instanceof z.core.$ZodTuple && JsonValue.isArray(value)) {
        const { items, rest } = definition._zod.def;

        return value.map((entry, index) => {
            const item = items[index] ?? rest;

            return item === null || item === undefined
                ? entry
                : (mapSensitive(item, entry, map) ?? null);
        });
    }
    // map each value of a record, leaving out values mapped to nothing
    else if (definition instanceof z.core.$ZodRecord && JsonValue.isObject(value)) {
        const values = definition._zod.def.valueType;

        return Object.fromEntries(
            Object.entries(value).flatMap(([key, entry]) => {
                const mapped = mapSensitive(values, entry, map);

                return mapped === undefined ? [] : [[key, mapped]];
            }),
        );
    }
    // map by every union option the value matches, refusing a value matching none
    else if (definition instanceof z.core.$ZodUnion) {
        const options = definition._zod.def.options.filter(
            (each) => z.safeParse(each, value).success,
        );
        if (options.length === 0) {
            throw new TypeError("a value matches no option of its union");
        }

        return options.reduce<JsonValue | undefined>(
            (mapped, option) => mapSensitive(option, mapped, map),
            value,
        );
    }
    // map by both sides of an intersection
    else if (definition instanceof z.core.$ZodIntersection) {
        const left = mapSensitive(definition._zod.def.left, value, map);

        return mapSensitive(definition._zod.def.right, left, map);
    }
    // look through optional, nullable, defaulted and read-only wrappers to the schema they wrap
    else if (isWrapper(definition)) {
        return mapSensitive(definition._zod.def.innerType, value, map);
    }
    // look through a lazy schema to the one it returns
    else if (definition instanceof z.core.$ZodLazy) {
        return mapSensitive(definition._zod.def.getter(), value, map);
    }

    return value;
}

/** Leave out an object's sensitive fields by its schema, handing each dropped value to `found`. */
export function redactFields(
    definition: z.ZodObject,
    value: JsonObject,
    found?: (sensitive: unknown) => void,
): JsonObject {
    return mapFields(definition, value, (marked) => {
        found?.(marked);

        return undefined;
    });
}

/** Replace the sensitive values in the fields of an object by what `map` makes of them. */
function mapFields(
    definition: z.core.$ZodObject,
    value: JsonObject,
    map: (value: JsonValue, sensitivity: Sensitivity) => JsonValue | undefined,
): JsonObject {
    const shape: Readonly<Record<string, z.core.$ZodType | undefined>> = definition._zod.def.shape;

    return Object.fromEntries(
        Object.entries(value).flatMap(([name, field]) => {
            const property = shape[name];
            const mapped = property === undefined ? field : mapSensitive(property, field, map);

            return mapped === undefined ? [] : [[name, mapped]];
        }),
    );
}

/** Mask a personal value: a list entry by entry, a record's every entry, a scalar whole. */
function maskOf(value: JsonValue): JsonValue {
    // mask a list entry by entry
    if (Array.isArray(value)) {
        return value.map(maskOf);
    }
    // mask each entry of a record
    else if (typeof value === "object" && value !== null) {
        return Object.fromEntries(Object.keys(value).map((key) => [key, MASK]));
    }
    // mask a scalar
    else {
        return MASK;
    }
}

/** Report whether a schema wraps another: optional, nullable, defaulted or read-only. */
function isWrapper(
    definition: z.core.$ZodType,
): definition is
    | z.core.$ZodOptional
    | z.core.$ZodNullable
    | z.core.$ZodDefault
    | z.core.$ZodPrefault
    | z.core.$ZodNonOptional
    | z.core.$ZodReadonly {
    return (
        definition instanceof z.core.$ZodOptional ||
        definition instanceof z.core.$ZodNullable ||
        definition instanceof z.core.$ZodDefault ||
        definition instanceof z.core.$ZodPrefault ||
        definition instanceof z.core.$ZodNonOptional ||
        definition instanceof z.core.$ZodReadonly
    );
}

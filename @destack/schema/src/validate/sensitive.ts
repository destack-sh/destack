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

/** The sensitive values a schema marks, found once to redact, mask or map its values. */
export class Redaction {
    /** Whether the schema marks a personal value at any depth. */
    readonly isPersonal: boolean;
    /** The paths to the schema's sensitive values, none when it marks none. */
    readonly #node: RedactionNode | undefined;

    /** Walk a schema once for the values it marks sensitive. */
    constructor(definition: z.core.$ZodType) {
        this.isPersonal = marksIn(
            definition,
            new Set(),
            (sensitivity) => sensitivity === "personal",
        );
        this.#node = nodeOf(definition, new Map());
    }

    /** Map each sensitive value, leaving out the values `map` returns none for. */
    map(
        value: JsonValue | undefined,
        map: (value: JsonValue, sensitivity: Sensitivity) => JsonValue | undefined,
    ): JsonValue | undefined {
        return mapNode(this.#node, value, map);
    }

    /** Leave out each sensitive value, handing each dropped value to `found`. */
    redact(
        value: JsonValue | undefined,
        found?: (sensitive: unknown) => void,
    ): JsonValue | undefined {
        return this.map(value, (marked) => {
            found?.(marked);

            return undefined;
        });
    }

    /** Leave out an object's sensitive fields, handing each dropped value to `found`. */
    redactFields(value: JsonObject, found?: (sensitive: unknown) => void): JsonObject {
        // leave an object whose schema marks nothing as it is
        const node = this.#node;
        if (node === undefined) {
            return value;
        }
        // refuse a schema that is not an object of fields
        else if (node.kind !== "object") {
            throw new TypeError("a redaction of fields needs an object schema");
        }

        return mapFields(node, value, (marked) => {
            found?.(marked);

            return undefined;
        });
    }

    /** Mask each personal value, leaving every other value as it is. */
    mask(value: JsonValue): JsonValue {
        // mask each personal value, which leaves every value present
        const masked = this.map(value, (marked, sensitivity) =>
            sensitivity === "personal" ? maskOf(marked) : marked,
        );
        if (masked === undefined) {
            throw new TypeError("masking skipped a value");
        }

        return masked;
    }
}

/** Where a schema marks sensitive values: a marked value, or a schema containing some. */
type RedactionNode =
    // a value marked whole
    | { readonly kind: "marked"; readonly sensitivity: Sensitivity }
    // an object's marking fields, by name
    | { readonly kind: "object"; readonly fields: Map<string, RedactionNode> }
    // an array's element, a tuple's items and rest, and a record's values
    | { readonly kind: "array"; readonly element: RedactionNode }
    | {
          readonly kind: "tuple";
          readonly items: readonly (RedactionNode | undefined)[];
          readonly rest: RedactionNode | undefined;
      }
    | { readonly kind: "record"; readonly value: RedactionNode }
    // a union's every option, and an intersection's marking sides
    | {
          readonly kind: "union";
          readonly options: readonly {
              readonly schema: z.core.$ZodType;
              readonly node: RedactionNode | undefined;
          }[];
      }
    | { readonly kind: "intersection"; readonly sides: readonly RedactionNode[] }
    // the schema a wrapper or a lazy schema returns, set once it is built
    | { readonly kind: "wrapped"; inner: RedactionNode | undefined };

/** Build the node of a schema marking a sensitive value, absent for one marking none. */
function nodeOf(
    definition: z.core.$ZodType,
    built: Map<z.core.$ZodType, RedactionNode | undefined>,
): RedactionNode | undefined {
    // reuse a schema's node, which recursive schemas return to, and leave a schema marking nothing
    const sensitivity = sensitivityOf(definition);
    if (built.has(definition)) {
        return built.get(definition);
    } else if (!marksIn(definition, new Set(), (marked) => marked !== undefined)) {
        built.set(definition, undefined);

        return undefined;
    }
    // take a marked schema whole
    else if (sensitivity !== undefined) {
        const node = { kind: "marked", sensitivity } as const;
        built.set(definition, node);

        return node;
    }

    // record objects and wrappers before their contents, as recursive schemas return to them
    if (definition instanceof z.core.$ZodObject) {
        const node = { kind: "object", fields: new Map<string, RedactionNode>() } as const;
        built.set(definition, node);
        for (const [name, field] of Object.entries<z.core.$ZodType>(definition._zod.def.shape)) {
            const child = nodeOf(field, built);
            if (child !== undefined) {
                node.fields.set(name, child);
            }
        }

        return node;
    } else if (isWrapper(definition) || definition instanceof z.core.$ZodLazy) {
        const node: RedactionNode = { kind: "wrapped", inner: undefined };
        built.set(definition, node);
        node.inner = nodeOf(
            isWrapper(definition) ? definition._zod.def.innerType : definition._zod.def.getter(),
            built,
        );

        return node;
    }

    // build the other schemas from what they contain
    const node = containerOf(definition, built);
    built.set(definition, node);

    return node;
}

/** Build the node of an array, tuple, record, union or intersection marking a sensitive value. */
function containerOf(
    definition: z.core.$ZodType,
    built: Map<z.core.$ZodType, RedactionNode | undefined>,
): RedactionNode | undefined {
    // record an array's element, a tuple's items and rest, and a record's values
    if (definition instanceof z.core.$ZodArray) {
        const element = nodeOf(definition._zod.def.element, built);

        return element === undefined ? undefined : { kind: "array", element };
    } else if (definition instanceof z.core.$ZodTuple) {
        const { items, rest } = definition._zod.def;

        return {
            kind: "tuple",
            items: items.map((item) => nodeOf(item, built)),
            rest: rest === null ? undefined : nodeOf(rest, built),
        };
    } else if (definition instanceof z.core.$ZodRecord) {
        const value = nodeOf(definition._zod.def.valueType, built);

        return value === undefined ? undefined : { kind: "record", value };
    }
    // record a union's every option, which values match, and an intersection's marking sides
    else if (definition instanceof z.core.$ZodUnion) {
        const options = definition._zod.def.options.map((option) => ({
            schema: option,
            node: nodeOf(option, built),
        }));

        return { kind: "union", options };
    } else if (definition instanceof z.core.$ZodIntersection) {
        const { left, right } = definition._zod.def;
        const sides = [nodeOf(left, built), nodeOf(right, built)].filter(
            (side) => side !== undefined,
        );

        return { kind: "intersection", sides };
    }

    return undefined;
}

/** Report whether a schema marks a value a predicate takes, walking each schema once. */
function marksIn(
    definition: z.core.$ZodType,
    seen: Set<z.core.$ZodType>,
    isMarked: (sensitivity: Sensitivity | undefined) => boolean,
): boolean {
    // stop at a schema already walked, as recursive lazy schemas return
    if (seen.has(definition)) {
        return false;
    }
    seen.add(definition);

    // read the schema's own mark, then the schemas it contains
    return (
        isMarked(sensitivityOf(definition)) ||
        contained(definition).some((child) => marksIn(child, seen, isMarked))
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

/** Replace each sensitive value a node marks by what `map` makes of it. */
function mapNode(
    node: RedactionNode | undefined,
    value: JsonValue | undefined,
    map: (value: JsonValue, sensitivity: Sensitivity) => JsonValue | undefined,
): JsonValue | undefined {
    // leave a value nothing marks, map a marked one, and answer an absent or null one as it is
    if (node === undefined || value === undefined) {
        return value;
    } else if (node.kind === "marked") {
        return map(value, node.sensitivity);
    } else if (value === null) {
        return value;
    }

    // map each field of an object, each element of an array and each item of a tuple
    if (node.kind === "object" && JsonValue.isObject(value)) {
        return mapFields(node, value, map);
    } else if (node.kind === "array" && JsonValue.isArray(value)) {
        return value.map((entry) => mapNode(node.element, entry, map) ?? null);
    } else if (node.kind === "tuple" && JsonValue.isArray(value)) {
        return value.map((entry, index) => {
            const item = index < node.items.length ? node.items[index] : node.rest;

            return item === undefined ? entry : (mapNode(item, entry, map) ?? null);
        });
    }
    // map each value of a record, leaving out values mapped to nothing
    else if (node.kind === "record" && JsonValue.isObject(value)) {
        return Object.fromEntries(
            Object.entries(value).flatMap(([key, entry]) => {
                const mapped = mapNode(node.value, entry, map);

                return mapped === undefined ? [] : [[key, mapped]];
            }),
        );
    }
    // map by every union option the value matches, refusing a value matching none
    else if (node.kind === "union") {
        const options = node.options.filter((option) => z.safeParse(option.schema, value).success);
        if (options.length === 0) {
            throw new TypeError("a value matches no option of its union");
        }

        return options.reduce<JsonValue | undefined>(
            (mapped, option) => mapNode(option.node, mapped, map),
            value,
        );
    }
    // map by each side of an intersection, and by the schema a wrapper returns
    else if (node.kind === "intersection") {
        return node.sides.reduce<JsonValue | undefined>(
            (mapped, side) => mapNode(side, mapped, map),
            value,
        );
    } else if (node.kind === "wrapped") {
        return mapNode(node.inner, value, map);
    }

    return value;
}

/** Replace the sensitive values in the fields of an object by what `map` makes of them. */
function mapFields(
    node: Extract<RedactionNode, { readonly kind: "object" }>,
    value: JsonObject,
    map: (value: JsonValue, sensitivity: Sensitivity) => JsonValue | undefined,
): JsonObject {
    return Object.fromEntries(
        Object.entries(value).flatMap(([name, field]) => {
            const mapped = mapNode(node.fields.get(name), field, map);

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

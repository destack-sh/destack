import type { JsonObject, JsonValue } from "@destack/schema";

/** The control of each JSON Schema string format a dedicated control takes. */
const STRING_FORMATS = new Map<string, SchemaTextField["control"]>([
    ["email", "email"],
    ["uri", "url"],
    ["date", "date"],
    ["time", "time"],
    ["date-time", "date-time"],
]);

/** The keywords whose values are data instead of schemas, which hold no references to check. */
const DATA_KEYWORDS = new Set(["const", "default", "enum", "examples"]);

/** What every field of a schema form knows of its property. */
interface SchemaFieldBase {
    /** The property's name, the item's index for an array item. */
    readonly name: string;
    /** The label, the schema's title or the property's name. */
    readonly title: string;
    /** The help text, absent without one. */
    readonly description?: string;
    /** Whether the property is required. */
    readonly isRequired: boolean;
    /** Whether the property takes null, which an emptied control holds. */
    readonly isNullable: boolean;
    /** The value the field starts from: its default, its constant or an empty object, list or null. */
    readonly start: JsonValue | undefined;
    /** The property's JSON Schema. */
    readonly schema: JsonObject;
}

/** A field typed as text: plain, an email address, a web address, a date, a time, a date and time, or JSON. */
export interface SchemaTextField extends SchemaFieldBase {
    /** The control. */
    readonly control: "text" | "email" | "url" | "date" | "time" | "date-time" | "json";
}

/** A field typed as a number within bounds. */
export interface SchemaNumberField extends SchemaFieldBase {
    /** The control. */
    readonly control: "number";
    /** The smallest value, absent without a bound. */
    readonly minimum?: number;
    /** The largest value, absent without a bound. */
    readonly maximum?: number;
    /** The step between values: the schema's multiple, 1 for integers, any otherwise. */
    readonly step: number | "any";
}

/** A field checked or unchecked. */
export interface SchemaBooleanField extends SchemaFieldBase {
    /** The control. */
    readonly control: "boolean";
}

/** A field choosing one value, or a list choosing several. */
export interface SchemaChoiceField extends SchemaFieldBase {
    /** The control: one choice or several. */
    readonly control: "choice" | "choices";
    /** The values to choose from, with their labels. */
    readonly choices: readonly SchemaChoice[];
}

/** A field holding an object, its properties as fields. */
export interface SchemaObjectField extends SchemaFieldBase {
    /** The control. */
    readonly control: "object";
    /** The fields of its properties, in property order. */
    readonly fields: readonly SchemaField[];
}

/** A field holding a list of items of one schema. */
export interface SchemaArrayField extends SchemaFieldBase {
    /** The control. */
    readonly control: "array";
    /** The field each item takes. */
    readonly item: SchemaField;
}

/** A field holding one of several kinds of object, chosen first. */
export interface SchemaVariantField extends SchemaFieldBase {
    /** The control. */
    readonly control: "variant";
    /** The kinds of object, in schema order. */
    readonly variants: readonly SchemaObjectField[];
}

/** A field holding one constant value, which the form keeps without a control. */
export interface SchemaConstantField extends SchemaFieldBase {
    /** The control. */
    readonly control: "constant";
}

/** One field of a schema form, by the control its property's schema takes. */
export type SchemaField =
    | SchemaTextField
    | SchemaNumberField
    | SchemaBooleanField
    | SchemaChoiceField
    | SchemaObjectField
    | SchemaArrayField
    | SchemaVariantField
    | SchemaConstantField;

/** A value to choose and its label. */
export interface SchemaChoice {
    /** The value. */
    readonly value: JsonValue;
    /** The label: the schema's title, else the value. */
    readonly label: string;
}

/** List the fields of an object's JSON Schema in property order, resolving local references against the root schema, none for a schema of anything else. */
export function schemaFields(described: JsonObject, root: JsonObject = described): SchemaField[] {
    return new SchemaScope(root).fields(described);
}

/** Read the field a property's JSON Schema takes, resolving local references against the root schema. */
export function schemaField(
    name: string,
    described: JsonObject,
    isRequired: boolean,
    root: JsonObject = described,
): SchemaField {
    return new SchemaScope(root).field(name, described, isRequired);
}

/** A root JSON Schema whose fields expand as they are read, with the local references inside it resolved. */
class SchemaScope {
    /** The schema references resolve against. */
    readonly root: JsonObject;
    /** The references whose fields' starts are being read, which a schema requiring itself reaches again. */
    readonly #starting: Set<string>;

    /** Hold a root schema, refusing it when a reference inside it does not resolve. */
    constructor(root: JsonObject) {
        this.root = root;
        this.#starting = new Set();
        this.#check(root);
    }

    /** List the fields of an object's schema, in property order. */
    fields(described: JsonObject): SchemaField[] {
        // read the referenced object's properties and the required ones
        const [object] = this.resolve(described);
        const properties = objectOf(object["properties"]) ?? {};
        const required = new Set(stringsOf(object["required"]));

        return Object.entries(properties).flatMap(([name, property]) => {
            const read = objectOf(property);

            return read === undefined ? [] : [this.field(name, read, required.has(name))];
        });
    }

    /** Read the field a property's schema takes, expanding objects, lists and kinds only as they are read. */
    field(name: string, described: JsonObject, isRequired: boolean): SchemaField {
        // read the referenced schema, the member of a nullable union and the facts every field shares
        const [resolved, reference] = this.resolve(described);
        const [property, isNullable] = this.#withoutNull(resolved);
        const types = new Set(stringsOf(property["type"]));
        const base = {
            name,
            title: stringOf(property["title"]) ?? name,
            ...descriptionOf(property),
            isRequired,
            isNullable: isNullable || types.has("null"),
            schema: resolved,
        };
        const fallback = property["default"] ?? (isRequired && base.isNullable ? null : undefined);
        const members = objectsOf(property["oneOf"] ?? property["anyOf"]).map(
            (member) => this.resolve(member)[0],
        );
        const choices = choicesOf(property, members);

        // hold a constant without a control
        if ("const" in property) {
            return { ...base, control: "constant", start: property["const"] };
        }
        // choose one value of an enumeration
        else if (choices !== undefined) {
            return { ...base, control: "choice", choices, start: fallback };
        }
        // choose one kind of object, then fill its fields
        else if (members.length > 0 && members.every((member) => member["type"] === "object")) {
            return this.#variantField(base, property, fallback, reference);
        }
        // fill the fields of an object
        else if (types.has("object") && "properties" in property) {
            return this.#objectField(base, property, reference);
        }
        // list several values of an enumeration, or items of one schema
        else if (types.has("array")) {
            return this.#arrayField(base, property, fallback);
        }
        // check a boolean
        else if (types.has("boolean")) {
            return {
                ...base,
                control: "boolean",
                start: fallback ?? (isRequired ? false : undefined),
            };
        }
        // type a number within its bounds
        else if (types.has("number") || types.has("integer")) {
            return {
                ...base,
                control: "number",
                ...boundsOf(property),
                step: numberOf(property["multipleOf"]) ?? (types.has("integer") ? 1 : "any"),
                start: fallback,
            };
        }
        // type text in the control its format takes
        else if (types.has("string")) {
            const control = STRING_FORMATS.get(stringOf(property["format"]) ?? "") ?? "text";

            return { ...base, control, start: fallback };
        }

        return { ...base, control: "json", start: fallback };
    }

    /**
     * Follow a schema's local reference to the schema it points at, keeping the keywords beside it.
     *
     * Return the schema and the pointer it was reached by, undefined for a schema without a reference.
     */
    resolve(described: JsonObject): [JsonObject, string | undefined] {
        // keep a schema without a reference
        const reference = described["$ref"];
        if (typeof reference !== "string") {
            return [described, undefined];
        }

        // follow references until a schema without one, refusing a loop
        const seen = new Set<string>();
        let pointer: JsonValue | undefined = reference;
        let target = described;
        while (typeof pointer === "string") {
            if (seen.has(pointer)) {
                throw new TypeError(`schema reference ${reference} refers to itself`);
            }
            seen.add(pointer);
            target = this.#pointed(pointer);
            pointer = target["$ref"];
        }

        return [{ ...target, ...withoutKeys(described, ["$ref"]) }, reference];
    }

    /** Read a field's start, refusing a schema whose start requires a start of itself. */
    start<Value>(reference: string | undefined, read: () => Value): Value {
        // read a start reached without a reference as it is
        if (reference === undefined) {
            return read();
        } else if (this.#starting.has(reference)) {
            throw new TypeError(`schema ${reference} requires a value of itself`);
        }

        // mark the reference while its start is read
        this.#starting.add(reference);
        try {
            return read();
        } finally {
            this.#starting.delete(reference);
        }
    }

    /** Read the field of a union of objects, its kinds expanding as they are read. */
    #variantField(
        base: Omit<SchemaFieldBase, "start">,
        described: JsonObject,
        fallback: JsonValue | undefined,
        reference: string | undefined,
    ): SchemaVariantField {
        // expand each kind once, starting a required union from its first kind
        let variants: SchemaObjectField[] | undefined;
        const variantsOf = (): SchemaObjectField[] =>
            (variants ??= objectsOf(described["oneOf"] ?? described["anyOf"]).map(
                (member, index) => {
                    // title each kind and expand its fields as they are read
                    const [kind, kindReference] = this.resolve(member);
                    const title = variantTitle(kind, index);
                    const kindBase = {
                        name: String(index),
                        title,
                        ...descriptionOf(kind),
                        isRequired: true,
                        isNullable: false,
                        schema: kind,
                    };

                    return this.#objectField(kindBase, kind, kindReference);
                },
            ));
        const startOf = (): JsonValue | undefined =>
            this.start(
                reference,
                () => fallback ?? (base.isRequired ? variantsOf()[0]?.start : undefined),
            );

        return {
            ...base,
            control: "variant",
            get variants() {
                return variantsOf();
            },
            get start() {
                return startOf();
            },
        };
    }

    /** Read the field of an object, its fields expanding as they are read. */
    #objectField(
        base: Omit<SchemaFieldBase, "start">,
        described: JsonObject,
        reference: string | undefined,
    ): SchemaObjectField {
        // expand the fields once, starting a required object from its fields' starts
        let fields: SchemaField[] | undefined;
        const fieldsOf = (): SchemaField[] => (fields ??= this.fields(described));
        const startOf = (): JsonValue | undefined =>
            this.start(
                reference,
                () => described["default"] ?? (base.isRequired ? emptyOf(fieldsOf()) : undefined),
            );

        return {
            ...base,
            control: "object",
            get fields() {
                return fieldsOf();
            },
            get start() {
                return startOf();
            },
        };
    }

    /** Read the field of a list, offering an enumeration's values or expanding its item as it is read. */
    #arrayField(
        base: Omit<SchemaFieldBase, "start">,
        described: JsonObject,
        fallback: JsonValue | undefined,
    ): SchemaChoiceField | SchemaArrayField {
        // offer the values of an enumeration of items
        const items = objectOf(described["items"]) ?? {};
        const itemChoices = choicesOf(this.resolve(items)[0], []);
        const start = fallback ?? (base.isRequired ? [] : undefined);
        if (itemChoices !== undefined) {
            return { ...base, control: "choices", choices: itemChoices, start };
        }

        // expand the item once
        let item: SchemaField | undefined;
        const itemOf = (): SchemaField => (item ??= this.field("", items, true));

        return {
            ...base,
            control: "array",
            get item() {
                return itemOf();
            },
            start,
        };
    }

    /** Split the null member off a nullable union, reporting whether there was one. */
    #withoutNull(described: JsonObject): [JsonObject, boolean] {
        // keep a schema that is no union with null
        const members = objectsOf(described["anyOf"] ?? described["oneOf"]).map(
            (member) => this.resolve(member)[0],
        );
        const others = members.filter((member) => member["type"] !== "null");
        if (others.length === members.length || others.length !== 1) {
            return [described, false];
        }

        // merge the one other member under the union's title and help
        const [other] = others;

        return [{ ...other, ...withoutKeys(described, ["anyOf", "oneOf"]) }, true];
    }

    /** Read the schema a local JSON Pointer points at, refusing one that points at none. */
    #pointed(pointer: string): JsonObject {
        // refuse a reference outside the root
        if (!pointer.startsWith("#")) {
            throw new TypeError(`schema reference ${pointer} is not local to the schema`);
        }

        // walk the pointer's tokens, unescaped as URI fragments and RFC 6901 escape them
        const tokens = decodeURIComponent(pointer.slice(1)).split("/").slice(1);
        let target: JsonValue | undefined = this.root;
        for (const token of tokens) {
            const key = token.replaceAll("~1", "/").replaceAll("~0", "~");
            target = Array.isArray(target) ? listOf(target)[Number(key)] : objectOf(target)?.[key];
        }

        // refuse a pointer that reaches no schema
        const found = objectOf(target);
        if (found === undefined) {
            throw new TypeError(`schema reference ${pointer} does not resolve`);
        }

        return found;
    }

    /** Resolve every reference inside a schema, refusing the first that does not resolve. */
    #check(described: JsonValue): void {
        // descend into lists
        const object = objectOf(described);
        if (object === undefined) {
            for (const entry of listOf(described)) {
                this.#check(entry);
            }

            return;
        }

        // resolve the object's reference and descend into its schemas, leaving data keywords out
        this.resolve(object);
        for (const [key, entry] of Object.entries(object)) {
            if (!DATA_KEYWORDS.has(key)) {
                this.#check(entry);
            }
        }
    }
}

/** Read the object an object field's fields start from, leaving out the ones without a start. */
export function emptyOf(fields: readonly SchemaField[]): JsonObject {
    return Object.fromEntries(
        fields.flatMap((field) => (field.start === undefined ? [] : [[field.name, field.start]])),
    );
}

/** Find the variant of a field an object is, by the constants it holds, absent when none matches. */
export function variantOf(
    field: SchemaVariantField,
    value: JsonValue | undefined,
): number | undefined {
    // match the first variant whose constants the value holds
    const held = objectOf(value);
    if (held === undefined) {
        return undefined;
    }
    const index = field.variants.findIndex((variant) =>
        variant.fields.every(
            (inner) =>
                inner.control !== "constant" ||
                JSON.stringify(held[inner.name]) === JSON.stringify(inner.start),
        ),
    );

    return index === -1 ? undefined : index;
}

/** Read the choices of an enumeration, or of a union of constants, absent for any other schema. */
function choicesOf(
    property: JsonObject,
    members: readonly JsonObject[],
): readonly SchemaChoice[] | undefined {
    // offer an enumeration's values
    const values = property["enum"];
    if (values !== undefined && values !== null && isArray(values)) {
        return values.map((value) => ({ value, label: labelOf(value) }));
    }
    // offer a union's constants under their titles
    else if (members.length > 0 && members.every((member) => "const" in member)) {
        return members.map((member) => ({
            value: member["const"] ?? null,
            label: stringOf(member["title"]) ?? labelOf(member["const"] ?? null),
        }));
    }

    return undefined;
}

/** Title a variant: its schema's title, else the constants it holds, else its position. */
function variantTitle(member: JsonObject, index: number): string {
    const constants = Object.values(objectOf(member["properties"]) ?? {}).flatMap((property) => {
        const read = objectOf(property);

        return read !== undefined && "const" in read ? [labelOf(read["const"] ?? null)] : [];
    });

    return stringOf(member["title"]) ?? (constants.join(" ") || String(index + 1));
}

/** Drop some keywords off a schema. */
function withoutKeys(described: JsonObject, keys: readonly string[]): JsonObject {
    return Object.fromEntries(Object.entries(described).filter(([key]) => !keys.includes(key)));
}

/** Label a value: a string as it is, anything else as JSON. */
function labelOf(value: JsonValue): string {
    return typeof value === "string" ? value : JSON.stringify(value);
}

/** Read a JSON value as an object, absent for anything else. */
export function objectOf(value: JsonValue | undefined): JsonObject | undefined {
    return typeof value === "object" && value !== null && !isArray(value) ? value : undefined;
}

/** Report whether a JSON value is an array, narrowing readonly arrays too. */
function isArray(value: JsonValue): value is readonly JsonValue[] {
    return Array.isArray(value);
}

/** Read a JSON value as a list, empty for anything else. */
export function listOf(value: JsonValue | undefined): readonly JsonValue[] {
    return value !== undefined && value !== null && isArray(value) ? value : [];
}

/** Read the objects of a JSON array, none for anything else. */
function objectsOf(value: JsonValue | undefined): JsonObject[] {
    return listOf(value).flatMap((entry) => {
        const read = objectOf(entry);

        return read === undefined ? [] : [read];
    });
}

/** Read a JSON value as a string, absent for anything else. */
function stringOf(value: JsonValue | undefined): string | undefined {
    return typeof value === "string" ? value : undefined;
}

/** Read a JSON value as a number, absent for anything else. */
function numberOf(value: JsonValue | undefined): number | undefined {
    return typeof value === "number" ? value : undefined;
}

/** Read a JSON value as strings: one string, or the strings of an array. */
function stringsOf(value: JsonValue | undefined): string[] {
    return [value ?? []].flat().filter((entry) => typeof entry === "string");
}

/** Read a schema's help text as an entry to spread, none without one. */
function descriptionOf(described: JsonObject): { readonly description?: string } {
    const description = stringOf(described["description"]);

    return description === undefined ? {} : { description };
}

/** Read a number schema's bounds as entries to spread, inclusive or exclusive alike. */
function boundsOf(described: JsonObject): { readonly minimum?: number; readonly maximum?: number } {
    const minimum = numberOf(described["minimum"] ?? described["exclusiveMinimum"]);
    const maximum = numberOf(described["maximum"] ?? described["exclusiveMaximum"]);

    return {
        ...(minimum === undefined ? {} : { minimum }),
        ...(maximum === undefined ? {} : { maximum }),
    };
}

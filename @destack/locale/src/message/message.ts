import { ModuleMetadata, PackageId } from "@destack/package";
import { defineSchema, type JsonValue, schema } from "@destack/schema";

/** An argument's name: a camel case MessageFormat 2 variable name. */
const ARGUMENT_NAME = /^[a-z][a-zA-Z0-9]*$/u;

/** The names of the positional placeholders a template's values take. */
const POSITIONAL_NAME = /^p[0-9]+$/u;

/** The plural categories CLDR defines, `other` always required. */
type PluralForms = Partial<Record<"zero" | "one" | "two" | "few" | "many", string>> & {
    /** The form for every other count. */
    readonly other: string;
};

/** The cases of a selection, `other` always required. */
type SelectCases = Readonly<Record<string, string>> & {
    /** The case for every other value. */
    readonly other: string;
};

/** The key of the argument names a message's readers supply, a type alone. */
declare const ARGUMENTS: unique symbol;

/** A localizable message: its identifier, MessageFormat 2 source and the JSON values it formats. */
export type Message<Names extends string = string> = schema.Infer<typeof messageSchema> & {
    /** The names of the arguments its readers supply when rendering it. */
    readonly [ARGUMENTS]?: Names;
};

/** A value a message's reader supplies by name when rendering it, such as a payload's field. */
export interface Argument<Name extends string = string> {
    /** The kind of interpolation. */
    readonly kind: "argument";
    /** The variable the value fills. */
    readonly name: Name;
}

/** A part of a message whose text varies with a value: a plural count or a selected case. */
export interface Variant<Value = number | string | Argument> {
    /** How the value selects the text. */
    readonly kind: "plural" | "select";
    /** The selecting value. */
    readonly value: Value;
    /** The text of each category or case, `#` standing for the value in a plural. */
    readonly forms: Readonly<Record<string, string>>;
}

/** The names of the arguments a template's interpolation leaves to the message's readers. */
type ArgumentName<Part> =
    Part extends Argument<infer Name>
        ? Name
        : Part extends Variant<Argument<infer Name>>
          ? Name
          : never;

/** A template tag writing messages, typed by the arguments their readers supply. */
export type MessageTag = <const Parts extends readonly unknown[]>(
    strings: TemplateStringsArray,
    ...parts: Parts
) => Message<ArgumentName<Parts[number]>>;

/** Leave a value to the message's reader, who supplies it by name when rendering the message. */
export function argument<Name extends string>(name: Name): Argument<Name> {
    // refuse a name that is no variable name, or one the positional placeholders take
    if (!ARGUMENT_NAME.test(name) || POSITIONAL_NAME.test(name)) {
        throw new TypeError(`message argument ${name} is no camel case name of its own`);
    }

    return { kind: "argument", name };
}

/** Vary a message's text with a count by its CLDR plural category, `#` standing for the count. */
export function plural<Value extends number | Argument>(
    value: Value,
    forms: PluralForms,
): Variant<Value> {
    return { kind: "plural", value, forms };
}

/** Vary a message's text with a value, `other` covering every unlisted one. */
export function select<Value extends string | Argument>(
    value: Value,
    cases: SelectCases,
): Variant<Value> {
    return { kind: "select", value, forms: cases };
}

/** Write a localizable message from a template, with interpolations as placeholders or variants. */
export const t = Object.assign(
    <const Parts extends readonly unknown[]>(
        strings: TemplateStringsArray,
        ...parts: Parts
    ): Message<ArgumentName<Parts[number]>> =>
        // refuse a template the module transform did not stamp
        Message.context("", ModuleMetadata.require(undefined, "t"))(strings, ...parts),
    {
        /** Build the template tag writing a module's messages, as the module transform calls it. */
        withModule(module: ModuleMetadata): MessageTag {
            return Message.context("", module);
        },
    },
);

/** Write text the build never extracts and no catalog translates, such as a brand or a code. */
export function verbatim(strings: TemplateStringsArray, ...parts: readonly unknown[]): string {
    return String.raw({ raw: strings }, ...parts);
}

/** A localizable message: its identifier, MessageFormat 2 source and the JSON values it formats. */
const messageSchema = defineSchema(
    schema.object({
        /** The package that wrote it, whose catalogs translate it. */
        package: PackageId,
        /** The stable identifier catalogs translate it under. */
        id: schema.string().min(1),
        /** The source text in the package's language, as MessageFormat 2. */
        source: schema.string(),
        /** The values of its placeholders and selectors, by variable name. */
        values: schema.jsonObject(),
    }),
);

/** The messages a package writes, rendered where they are read. */
export const Message = Object.assign(messageSchema, {
    /** Build a message of a module's package from a template's text and interpolations. */
    of(
        strings: readonly string[],
        parts: readonly unknown[],
        context: string,
        module: ModuleMetadata,
    ): schema.Infer<typeof messageSchema> {
        // name each interpolation: an argument by its own name, any other value by its position
        const values: Record<string, JsonValue> = {};
        const selectors: { readonly name: string; readonly variant: Variant }[] = [];
        const pieces: (string | { readonly name: string; readonly variant?: Variant })[] = [];
        for (const [index, text] of strings.entries()) {
            pieces.push(text);
            if (index < parts.length) {
                const part = parts[index];
                const value = isVariant(part) ? part.value : part;
                const name = isArgument(value) ? value.name : `p${index}`;
                if (!isArgument(value)) {
                    values[name] = jsonOf(name, value);
                }

                // place the variable, selecting by a variant's value
                pieces.push(isVariant(part) ? { name, variant: part } : { name });
                if (isVariant(part)) {
                    selectors.push({ name, variant: part });
                }
            }
        }

        // write the MessageFormat 2 source, one variant per combination of selector keys
        const source = selectors.length === 0 ? pattern(pieces, {}) : matcher(pieces, selectors);

        return { package: module.package.id, id: identify(source, context), source, values };
    },

    /** Build a template tag that writes a module's messages under a context. */
    context(context: string, module?: ModuleMetadata): MessageTag {
        // refuse a context the module transform did not stamp
        const stamped = ModuleMetadata.require(module, "Message.context");

        return (strings, ...parts) => Message.of(strings, parts, context, stamped);
    },
});

/** Require a message value to be JSON, which the message travels as. */
function jsonOf(name: string, value: unknown): JsonValue {
    const parsed = schema.json().safeParse(value);
    if (!parsed.success) {
        throw new TypeError(`message value ${name} is no JSON value`);
    }

    return parsed.data;
}

/** Report whether an interpolation is an argument the message's reader supplies. */
function isArgument(part: unknown): part is Argument {
    return typeof part === "object" && part !== null && "kind" in part && part.kind === "argument";
}

/** Report whether an interpolation is a plural or selected variant. */
function isVariant(part: unknown): part is Variant {
    return (
        typeof part === "object" &&
        part !== null &&
        "kind" in part &&
        (part.kind === "plural" || part.kind === "select") &&
        "forms" in part
    );
}

/** Write a matcher over the selectors, with a variant for every combination of their keys. */
function matcher(
    pieces: readonly (string | { readonly name: string; readonly variant?: Variant })[],
    selectors: readonly { readonly name: string; readonly variant: Variant }[],
): string {
    // declare each selector's function, plural counts as numbers
    const declarations = selectors.map(({ name, variant }) =>
        variant.kind === "plural" ? `.input {$${name} :number}` : `.input {$${name} :string}`,
    );

    // write one variant per combination of keys, the catch-all key last
    const keys = selectors.map(({ variant }) =>
        Object.keys(variant.forms).toSorted(byCatchAllLast),
    );
    const variants = combinations(keys).map((combination) => {
        const chosen = Object.fromEntries(
            selectors.map(({ name }, index) => [name, combination[index] ?? "other"]),
        );
        const label = combination.map((key) => (key === "other" ? "*" : key)).join(" ");

        return `${label} {{${pattern(pieces, chosen)}}}`;
    });

    return [
        ...declarations,
        `.match ${selectors.map(({ name }) => `$${name}`).join(" ")}`,
        ...variants,
    ].join("\n");
}

/** Write a pattern from text and placeholders, each variant taking its chosen form. */
function pattern(
    pieces: readonly (string | { readonly name: string; readonly variant?: Variant })[],
    chosen: Readonly<Record<string, string>>,
): string {
    return pieces
        .map((piece) => {
            // escape literal text
            if (typeof piece === "string") {
                return escape(piece);
            }
            // place a plain value
            else if (piece.variant === undefined) {
                return `{$${piece.name}}`;
            }

            // place the chosen form, # standing for a plural's count
            const form = piece.variant.forms[chosen[piece.name] ?? "other"] ?? "";

            return escape(form).replaceAll("#", `{$${piece.name}}`);
        })
        .join("");
}

/** Escape the characters MessageFormat 2 reserves in text. */
function escape(text: string): string {
    return text.replace(/[\\{}|]/gu, (character) => `\\${character}`);
}

/** Order keys with the catch-all `other` last. */
function byCatchAllLast(left: string, right: string): number {
    return Number(left === "other") - Number(right === "other");
}

/** List every combination of one key from each list. */
function combinations(lists: readonly (readonly string[])[]): string[][] {
    return lists.reduce<string[][]>(
        (combined, keys) => combined.flatMap((prefix) => keys.map((key) => [...prefix, key])),
        [[]],
    );
}

/** Identify a message by its source and context: a 64-bit FNV-1a hash in base 36. */
function identify(source: string, context: string): string {
    let hash = 0xcbf29ce484222325n;
    for (const byte of new TextEncoder().encode(`${context}\u0000${source}`)) {
        hash = BigInt.asUintN(64, (hash ^ BigInt(byte)) * 0x100000001b3n);
    }

    return hash.toString(36);
}

import { ModuleMetadata, PackageId } from "@destack/package";
import { defineSchema, type JsonValue, schema } from "@destack/schema";

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

/** A localizable message: its identifier, its MessageFormat 2 source and the JSON values it formats. */
export type Message = schema.Infer<typeof messageSchema>;

/** A part of a message whose text varies with a value: a plural count or a selected case. */
export interface Variant {
    /** How the value selects the text. */
    readonly kind: "plural" | "select";
    /** The selecting value. */
    readonly value: number | string;
    /** The text of each category or case, `#` standing for the value in a plural. */
    readonly forms: Readonly<Record<string, string>>;
}

/** Vary a message's text with a count by its CLDR plural category, `#` standing for the count. */
export function plural(value: number, forms: PluralForms): Variant {
    return { kind: "plural", value, forms };
}

/** Vary a message's text with a value, `other` covering every unlisted one. */
export function select(value: string, cases: SelectCases): Variant {
    return { kind: "select", value, forms: cases };
}

/** Write a localizable message from a template, its interpolations becoming placeholders or variants. */
export function t(strings: TemplateStringsArray, ...parts: readonly unknown[]): Message;
/** Build the template tag writing a module's messages, as the module transform calls it. */
export function t(
    module: ModuleMetadata,
): (strings: TemplateStringsArray, ...parts: readonly unknown[]) => Message;
/**
 * Build the writing module's template tag, refusing a template written without it.
 *
 * @construct the module transform rewrites each t`…` into t(module)`…`, so a template reaches t only untransformed.
 */
export function t(
    first: TemplateStringsArray | ModuleMetadata,
): Message | ((strings: TemplateStringsArray, ...parts: readonly unknown[]) => Message) {
    // refuse a template the module transform did not stamp
    const module = ModuleMetadata.require("package" in first ? first : undefined, "t");

    return Message.context("", module);
}

/** Write text the build never extracts and no catalog translates, such as a brand or a code. */
export function verbatim(strings: TemplateStringsArray, ...parts: readonly unknown[]): string {
    return String.raw({ raw: strings }, ...parts);
}

/** A localizable message: its identifier, its MessageFormat 2 source and the JSON values it formats, rendered where it is read. */
const messageSchema = defineSchema(
    schema.object({
        /** The package that wrote it, whose catalogs translate it. */
        package: PackageId,
        /** The stable identifier catalogs translate it under. */
        id: schema.string().min(1),
        /** The source text in the package's language, as MessageFormat 2. */
        source: schema.string(),
        /** The values of its placeholders and selectors, by variable name. */
        values: schema.record(schema.string(), schema.json()),
    }),
);

/** The messages a package writes, rendered where they are read, such as a notification in each recipient's locale. */
export const Message = Object.assign(messageSchema, {
    /** Build a message of a module's package from a template's text and interpolations under a disambiguating context. */
    of(
        strings: readonly string[],
        parts: readonly unknown[],
        context: string,
        module: ModuleMetadata,
    ): Message {
        // name each interpolation as a variable holding a JSON value, variants as selectors
        const values: Record<string, JsonValue> = {};
        const selectors: { readonly name: string; readonly variant: Variant }[] = [];
        const pieces: (string | { readonly name: string; readonly variant?: Variant })[] = [];
        for (const [index, text] of strings.entries()) {
            pieces.push(text);
            if (index < parts.length) {
                const name = `p${index}`;
                const part = parts[index];
                values[name] = jsonOf(name, isVariant(part) ? part.value : part);
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

    /** Build a template tag writing a module's messages under a context, the module passed by the module transform. */
    context(
        context: string,
        module?: ModuleMetadata,
    ): (strings: TemplateStringsArray, ...parts: readonly unknown[]) => Message {
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

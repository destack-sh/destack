import { schema } from "@destack/schema";

/** The prefix of every custom property a theme sets. */
export const CUSTOM_PROPERTY_PREFIX = "--destack";

/** A custom property a theme sets, such as `--destack-space-4`. */
export type CustomProperty = `${typeof CUSTOM_PROPERTY_PREFIX}-${string}`;

/** A token's type, as the design token format names it. */
export const TokenType = schema.enum([
    "color",
    "dimension",
    "fontFamily",
    "fontWeight",
    "duration",
    "cubicBezier",
    "shadow",
    "typography",
]);
/** A token's type. */
export type TokenType = schema.Infer<typeof TokenType>;

/** The prefix of the name of a reference's schema, the token type following it. */
export const REFERENCE_PREFIX = "style.reference.";

/** A token's path in braces, such as `{space.4}`. */
const REFERENCE = /^\{[A-Za-z][A-Za-z0-9]*(?:\.[A-Za-z0-9]+)*\}$/u;

/** A reference to a token by its path, such as `{space.4}`, as design tokens write one. */
const referenceJson = schema
    .string()
    .regex(REFERENCE, { message: "a reference is a token's path in braces, such as {space.4}" });
/** A reference to a token by its path. */
export type Reference = schema.Infer<typeof referenceJson>;

/** The schema of a reference to a token of each type, named so an input offers its tokens. */
const REFERENCES = {
    color: named("color"),
    dimension: named("dimension"),
    fontFamily: named("fontFamily"),
    fontWeight: named("fontWeight"),
    duration: named("duration"),
    cubicBezier: named("cubicBezier"),
    shadow: named("shadow"),
    typography: named("typography"),
} satisfies Record<TokenType, typeof referenceJson>;

/** A value of a design type, or a reference to a token of that type in its place. */
export type Bindable<Value> = Value | Reference;

/** Read the CSS value a referenced token has where it is drawn, such as `oklch(0.6 0.1 250)`. */
export type Resolve = (reference: Reference) => string;

/** A reference to a token by its path, such as `{space.4}`, as design tokens write one. */
export const Reference = Object.assign(referenceJson, {
    /** Type a reference to a token of a type, so an input offers that type's tokens. */
    of(type: TokenType) {
        return REFERENCES[type];
    },

    /** Take a value of a type, or a reference to a token of that type in its place. */
    or<Value extends schema.Schema>(value: Value, type: TokenType) {
        return schema.union([value, REFERENCES[type]]);
    },

    /** Write a value as CSS, a reference as its token's custom property. */
    write<Value>(value: Bindable<Value>, css: (value: Value) => string): string {
        return Reference.is(value) ? Reference.css(value) : css(value);
    },

    /** Tell whether a value is a reference, rather than the value itself. */
    is(value: unknown): value is Reference {
        return typeof value === "string" && REFERENCE.test(value);
    },

    /** Write a reference to the token at a path, such as `{space.4}`. */
    to(path: readonly string[]): Reference {
        return `{${path.join(".")}}`;
    },

    /** Read the path of the token a reference points at, such as `["space", "4"]`. */
    path(reference: Reference): readonly string[] {
        return reference.slice(1, -1).split(".");
    },

    /** Name the custom property of a token's path, such as `--destack-color-muted-foreground`. */
    property(path: readonly string[]): CustomProperty {
        const words = path.map((name) =>
            name.replaceAll(/[A-Z]/gu, (letter) => `-${letter.toLowerCase()}`),
        );

        return `${CUSTOM_PROPERTY_PREFIX}-${words.join("-")}`;
    },

    /** Write a reference as CSS reads it: the custom property the theme sets for its token. */
    css(reference: Reference): string {
        return `var(${Reference.property(Reference.path(reference))})`;
    },
});

/** Name the schema of a reference to a token of a type, its type after the prefix. */
function named(type: TokenType) {
    return referenceJson.meta({ id: `${REFERENCE_PREFIX}${type}`, title: "Token" });
}

import { type JsonObject, JsonValue, LocalReference } from "@destack/schema";
import { REFERENCE_PREFIX, TokenType } from "@destack/style";

/** The name the kit's input of a value or a token reference loads under. */
export const BINDABLE = "style.bindable";

/** A schema that takes a value, or a reference to a token of a type in its place. */
export interface Binding {
    /** The type of token it takes a reference to. */
    readonly type: TokenType;
    /** The description of the value itself, without the reference. */
    readonly literal: JsonObject;
}

/** A schema that takes a value, or a reference to a token of a type in its place. */
export const Binding = {
    /** Read the binding of a description, as style's `Reference.or` writes one, none for others. */
    of(description: JsonObject): Binding | undefined {
        // take two members, one a reference to a token's schema
        const members = description["anyOf"];
        if (members === undefined || !JsonValue.isArray(members) || members.length !== 2) {
            return undefined;
        }
        const types = members.map((member) => Binding.reference(member));
        const index = types.findIndex((type) => type !== undefined);
        const type = types[index];
        const literal = members[1 - index];

        // keep the document's definitions beside the value's own description
        if (type === undefined || literal === undefined || !JsonValue.isObject(literal)) {
            return undefined;
        }
        const { anyOf: _members, ...document } = description;

        return { type, literal: { ...document, ...literal } };
    },

    /** Read the token type a member refers to the reference schema of, none for others. */
    reference(member: unknown): TokenType | undefined {
        // take a reference to a definition named by the prefix
        const reference =
            typeof member === "object" && member !== null && "$ref" in member
                ? member.$ref
                : undefined;
        const name = typeof reference === "string" ? LocalReference.definition(reference) : null;

        return name?.startsWith(REFERENCE_PREFIX) === true
            ? TokenType.safeParse(name.slice(REFERENCE_PREFIX.length)).data
            : undefined;
    },
};

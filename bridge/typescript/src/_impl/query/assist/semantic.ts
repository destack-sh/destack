import type { SemanticTokenModifiers } from "../../../_generated/query/assist/semantic.js";

export const SemanticTokenModifiersImpl = {
    /** Return the union of two modifier sets. */
    union(left: SemanticTokenModifiers, right: SemanticTokenModifiers): SemanticTokenModifiers {
        return left | right;
    },

    /** Return whether all modifier bits from another set are present. */
    contains(modifiers: SemanticTokenModifiers, other: SemanticTokenModifiers): boolean {
        return (modifiers & other) === other;
    },

    /** Return the raw modifier bits. */
    bits(modifiers: SemanticTokenModifiers): number {
        return modifiers;
    },
};

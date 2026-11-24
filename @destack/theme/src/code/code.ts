import * as style from "@destack/style";
import { color, swatch, weight } from "../token/tokens.stylex.ts";

/** The look of each kind of token in code, by `TokenKind`, the rest in the color around them. */
export const code = style.create({
    /** A comment. */
    comment: { color: color.mutedForeground, fontStyle: "italic" },
    /** What separates and encloses, such as a brace. */
    punctuation: { color: color.mutedForeground },
    /** An operator, such as `??`. */
    operator: { color: color.mutedForeground },
    /** A keyword, such as `const`. */
    keyword: { color: swatch.purpleText },
    /** A value with a name, such as `true`. */
    constant: { color: swatch.orangeText },
    /** A number. */
    number: { color: swatch.orangeText },
    /** A string. */
    string: { color: swatch.greenText },
    /** A function as it is declared or called. */
    function: { color: swatch.blueText },
    /** A type or a class. */
    type: { color: swatch.yellowText },
    /** An element. */
    tag: { color: swatch.redText },
    /** An attribute of an element. */
    attribute: { color: swatch.yellowText },
    /** A heading. */
    heading: { color: swatch.blueText, fontWeight: weight.semibold },
    /** A link. */
    link: { color: swatch.blueText, textDecorationLine: "underline" },
    /** Strong text. */
    strong: { fontWeight: weight.semibold },
    /** Emphasized text. */
    emphasis: { fontStyle: "italic" },
    /** A line that was added. */
    inserted: { color: swatch.greenText },
    /** A line that was removed. */
    deleted: { color: swatch.redText },
});

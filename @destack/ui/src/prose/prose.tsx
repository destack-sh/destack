import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color } from "@destack/theme/tokens.stylex";
import { type JSX } from "@destack/view";
import "./prose.css";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The longest line of running text, in characters, the measure long-form text reads best at. */
const MEASURE = "65ch";

/** The leading of running text, Tailwind Typography's 28px over 16px, looser than the interface's text styles. */
const LEADING = 1.75;

/** The styles of a prose container; its elements take theirs from `prose.css`. */
const styles = style.create({
    prose: {
        color: color.foreground,
        lineHeight: LEADING,
        maxInlineSize: MEASURE,
        minInlineSize: 0,
    },
});

/** The properties of prose, the native element's attributes included. */
export type ProseProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** Style rendered HTML, such as converted Markdown, by its semantic elements: headings, lists, tables, code, quotes, figures, footnotes and alerts. */
export function Prose(properties: ProseProperties): JSX.Element {
    return renderPart("div", "prose", properties, [text.body, styles.prose]);
}

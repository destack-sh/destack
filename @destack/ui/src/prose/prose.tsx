import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color } from "@destack/theme/tokens.stylex";
import { type JSX, omit } from "@destack/view";
import "./prose.css";

/** The longest line of running text, in characters, the measure long-form text reads best at. */
const MEASURE = "65ch";

/** The styles of a prose container; its elements take theirs from `prose.css`. */
const styles = style.create({
    prose: {
        color: color.foreground,
        maxInlineSize: MEASURE,
        minInlineSize: 0,
    },
});

/** The properties of prose, the native element's attributes included. */
export interface ProseProperties extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** The StyleX styles applied after the container's styles. */
    readonly xstyle?: style.Styles;
}

/** Style rendered HTML, such as converted Markdown, by its semantic elements: headings, lists, tables, code, quotes, figures, footnotes and alerts. */
export function Prose(properties: ProseProperties): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            data-slot="prose"
            {...rest}
            {...style.attributes([text.body, styles.prose, properties.xstyle], properties.style)}
        />
    );
}

import { swatches } from "@destack/theme/swatch";
import { renderPart, type Render } from "../part/index.ts";
import type { Hue } from "@destack/color";
import * as style from "@destack/style";
import { color, radius, space } from "@destack/theme/tokens.stylex";
import { type JSX, omit, Show } from "@destack/view";
import { Glyph, type GlyphValue } from "../glyph/index.ts";

/** The styles of an object's chip. */
const styles = style.create({
    chip: {
        display: "inline-flex",
        alignItems: "center",
        gap: space[1],
        maxInlineSize: "100%",
        verticalAlign: "bottom",
        lineHeight: "inherit",
        color: color.foreground,
        textDecoration: "none",
    },
    glyph: {
        width: "1.2em",
        height: "1.2em",
        fontSize: "1em",
    },
    title: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        unicodeBidi: "plaintext",
        whiteSpace: "nowrap",
    },
    dot: (fill: string) => ({
        inlineSize: space[2],
        blockSize: space[2],
        flexShrink: 0,
        borderRadius: radius.full,
        backgroundColor: fill,
    }),
});

/** The properties of an object's chip. */
export interface ObjectChipProperties extends Omit<
    JSX.HTMLAttributes<HTMLElement>,
    "class" | "title" | "color" | "ref"
> {
    /** The object's title. */
    readonly title: string;
    /** The object's icon. */
    readonly glyph?: GlyphValue;
    /** The object's color. */
    readonly color?: Hue;
    /** The address of the object's page. */
    readonly href?: string;
    /** The StyleX styles applied after the chip's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
}

/** Render a reference to an object: its icon, title and accent, linking to its page. */
export function ObjectChip(properties: ObjectChipProperties): JSX.Element {
    // show the icon, the title and the accent's dot, on a link where the object has a page
    const rest = omit(properties, "title", "glyph", "color", "href", "children");
    const body = (
        <>
            <Show when={properties.glyph}>
                {(glyph) => <Glyph glyph={glyph()} size="sm" xstyle={styles.glyph} />}
            </Show>
            <span {...style.attrs(styles.title)}>{properties.title}</span>
            <Show when={properties.color}>
                {(accent) => <span {...style.attrs(styles.dot(swatches[accent()].mark))} />}
            </Show>
        </>
    );

    return (
        <Show
            when={properties.href}
            fallback={renderPart("span", "object-chip", rest, styles.chip, {
                get children() {
                    return body;
                },
            })}
        >
            {(href) =>
                renderPart("a", "object-chip", rest, styles.chip, {
                    get href() {
                        return href();
                    },
                    get children() {
                        return body;
                    },
                })
            }
        </Show>
    );
}

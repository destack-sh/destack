import type { Hue } from "@destack/color";
import * as style from "@destack/style";
import { color, radius, space, swatch } from "@destack/theme/tokens.stylex";
import { type JSX, omit, Show } from "@destack/view";
import { Glyph, type GlyphValue } from "../glyph/index.ts";

/** The styles of an object's chip. */
const styles = style.create({
    chip: {
        display: "inline-flex",
        alignItems: "center",
        gap: space[1],
        maxInlineSize: "100%",
        color: color.foreground,
        textDecoration: "none",
    },
    title: {
        overflow: "hidden",
        textOverflow: "ellipsis",
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
}

/** Render a reference to an object: its icon, title and accent, linking to its page. */
export function ObjectChip(properties: ObjectChipProperties): JSX.Element {
    // show the icon, the title and the accent's dot, on a link where the object has a page
    const rest = omit(properties, "title", "glyph", "color", "href", "xstyle", "style");
    const body = (
        <>
            <Show when={properties.glyph}>{(glyph) => <Glyph glyph={glyph()} size="sm" />}</Show>
            <span {...style.attrs(styles.title)}>{properties.title}</span>
            <Show when={properties.color}>
                {(accent) => <span {...style.attrs(styles.dot(swatch[`${accent()}Mark`]))} />}
            </Show>
        </>
    );

    return (
        <Show
            when={properties.href}
            fallback={
                <span
                    data-slot="object-chip"
                    {...rest}
                    {...style.attributes([styles.chip, properties.xstyle], properties.style)}
                >
                    {body}
                </span>
            }
        >
            {(href) => (
                <a
                    data-slot="object-chip"
                    href={href()}
                    {...rest}
                    {...style.attributes([styles.chip, properties.xstyle], properties.style)}
                >
                    {body}
                </a>
            )}
        </Show>
    );
}

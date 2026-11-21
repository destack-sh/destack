import type { SwatchName } from "@destack/color";
import * as style from "@destack/style";
import { color, radius, space, swatch } from "@destack/theme/tokens.stylex";
import { type JSX, Show } from "@destack/view";
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
        inlineSize: "0.5rem",
        blockSize: "0.5rem",
        flexShrink: 0,
        borderRadius: radius.full,
        backgroundColor: fill,
    }),
});

/** The properties of an object's chip. */
export interface ObjectChipProperties {
    /** The object's title. */
    readonly title: string;
    /** The object's icon. */
    readonly glyph?: GlyphValue;
    /** The object's accent. */
    readonly color?: SwatchName;
    /** The address of the object's page. */
    readonly href?: string;
}

/** Render a reference to an object: its icon, title and accent, linking to its page. */
export function ObjectChip(properties: ObjectChipProperties): JSX.Element {
    const body = (
        <>
            <Show when={properties.glyph}>{(glyph) => <Glyph glyph={glyph()} size="sm" />}</Show>
            <span {...style.attrs(styles.title)}>{properties.title}</span>
            <Show when={properties.color}>
                {(accent) => <span {...style.attrs(styles.dot(swatch[`${accent()}Solid`]))} />}
            </Show>
        </>
    );

    return (
        <Show
            when={properties.href}
            fallback={
                <span data-slot="object-chip" {...style.attrs(styles.chip)}>
                    {body}
                </span>
            }
        >
            {(href) => (
                <a data-slot="object-chip" href={href()} {...style.attrs(styles.chip)}>
                    {body}
                </a>
            )}
        </Show>
    );
}

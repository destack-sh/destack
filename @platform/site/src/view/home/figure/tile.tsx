import * as style from "@destack/style";

import { palette } from "../../palette.stylex";
import { AgentAvatar, isAgent } from "./agent";
import { paperGlyphOf } from "./planet";

/** Draw a Destack app, service or resource as an app icon: its white glyph on the sky. */
export function Tile(properties: { icon: string; size: "small" | "large" }) {
    // show your agent as its avatar, every other app or service as its glyph on the sky
    if (isAgent(properties.icon)) {
        return <AgentAvatar size={properties.size} />;
    }

    return (
        <span
            {...style.attrs(
                styles.tile,
                styles.tileBrand,
                properties.size === "large" && styles.tileLarge,
            )}
        >
            <span
                style={{
                    "mask-image": `url(/diagram/fill/${properties.icon}.svg)`,
                    "background-color": palette.tileGlyph,
                }}
                {...style.attrs(styles.glyph, properties.size === "large" && styles.glyphLarge)}
            />
        </span>
    );
}

/** Draw a Destack app or service inside a page: its glyph alone, in its colour. */
export function ServiceGlyph(properties: { icon: string }) {
    // show your agent as its avatar, every other app or service as its glyph
    if (isAgent(properties.icon)) {
        return <AgentAvatar size="glyph" />;
    }

    return (
        <span
            aria-hidden="true"
            style={{
                "mask-image": `url(/diagram/${properties.icon}.svg)`,
                "background-color": paperGlyphOf(properties.icon),
            }}
            {...style.attrs(styles.glyph, styles.serviceGlyph)}
        />
    );
}

/** The tile styles. */
const styles = style.create({
    tile: {
        alignItems: "center",
        borderRadius: "0.3125rem",
        boxShadow: `inset 0 0 0 1px color-mix(in srgb, ${palette.ink} 10%, transparent)`,
        display: "inline-flex",
        flexShrink: 0,
        height: "1.25rem",
        justifyContent: "center",
        width: "1.25rem",
    },
    tileBrand: {
        backgroundImage: palette.skyField,
        boxShadow: `inset 0 1px 0 color-mix(in srgb, ${palette.cream} 14%, transparent), inset 0 0 0 0.5px color-mix(in srgb, ${palette.cream} 10%, transparent), 0 1px 2px color-mix(in srgb, ${palette.ink} 25%, transparent)`,
    },
    tileLarge: {
        borderRadius: "0.4375rem",
        height: "1.75rem",
        width: "1.75rem",
    },
    glyph: {
        height: "0.875rem",
        width: "0.875rem",
        maskPosition: "center",
        maskRepeat: "no-repeat",
        maskSize: "contain",
    },
    serviceGlyph: {
        display: "inline-block",
        flexShrink: 0,
        height: "1rem",
        width: "1rem",
    },
    glyphLarge: {
        height: "1.1875rem",
        width: "1.1875rem",
    },
});

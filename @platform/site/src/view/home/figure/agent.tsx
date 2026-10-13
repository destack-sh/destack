import * as style from "@destack/style";

import { palette } from "../../palette.stylex";

/** The icon name that stands for your agent, drawn as its avatar instead of a glyph. */
const AGENT = "agent";

/** Report whether an icon stands for your agent, which shows as its avatar instead of a glyph. */
export function isAgent(icon: string): boolean {
    return icon === AGENT;
}

/** Draw your agent as a member of your space: a flat violet disc with its initial. */
export function AgentAvatar(properties: { size: "glyph" | "small" | "large" | "fill" }) {
    return (
        <span aria-hidden="true" {...style.attrs(styles.avatar, sizes[properties.size])}>
            A
        </span>
    );
}

/** The avatar's sizes: an in-page glyph, a small or large tile, or its container's full size. */
const sizes = style.create({
    glyph: { fontSize: "0.5625rem", height: "1rem", width: "1rem" },
    small: { fontSize: "0.6875rem", height: "1.25rem", width: "1.25rem" },
    large: { fontSize: "0.875rem", height: "1.75rem", width: "1.75rem" },
    fill: { fontSize: "inherit", height: "100%", width: "100%" },
});

/** The avatar styles. */
const styles = style.create({
    avatar: {
        alignItems: "center",
        backgroundColor: palette.violet,
        borderRadius: "50%",
        color: palette.tileGlyph,
        display: "inline-flex",
        flexShrink: 0,
        fontWeight: 700,
        justifyContent: "center",
        lineHeight: 1,
    },
});

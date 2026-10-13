import * as style from "@destack/style";

import { palette } from "../../palette.stylex";

/** The icon name that stands for your agent, drawn as its avatar instead of a glyph. */
const AGENT = "agent";

/** Report whether an icon stands for your agent, which shows as its avatar instead of a glyph. */
export function isAgent(icon: string): boolean {
    return icon === AGENT;
}

/** The agent you connected to your space, drawn by its own mark: the model it runs on. */
export const MODEL = { icon: "claude", tint: "#d97757", name: "Claude" } as const;

/** Draw your agent as the agent it is: its model's mark, white on the model's colour. */
export function AgentAvatar(properties: { size: "glyph" | "small" | "large" | "fill" }) {
    return (
        <span
            aria-hidden="true"
            title={MODEL.name}
            style={{ "background-color": MODEL.tint }}
            {...style.attrs(styles.avatar, sizes[properties.size])}
        >
            <span
                style={{ "mask-image": `url(/diagram/${MODEL.icon}.svg)` }}
                {...style.attrs(styles.mark)}
            />
        </span>
    );
}

/** The avatar's sizes: an in-page glyph, a small or large tile, or its container's full size. */
const sizes = style.create({
    glyph: { height: "1rem", width: "1rem" },
    small: { height: "1.25rem", width: "1.25rem" },
    large: { height: "1.75rem", width: "1.75rem" },
    fill: { height: "100%", width: "100%" },
});

/** The avatar styles. */
const styles = style.create({
    avatar: {
        alignItems: "center",
        borderRadius: "50%",
        display: "inline-flex",
        flexShrink: 0,
        justifyContent: "center",
    },
    mark: {
        backgroundColor: palette.tileGlyph,
        height: "62%",
        maskPosition: "center",
        maskRepeat: "no-repeat",
        maskSize: "contain",
        width: "62%",
    },
});

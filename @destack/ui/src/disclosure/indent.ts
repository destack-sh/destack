import * as style from "@destack/style";
import { color, space, stroke } from "@destack/theme/tokens.stylex";

/** The indent of a nested row per level of depth, which the theme scales. */
export const INDENT = space[4];

/** The styles of nested rows: their indent by depth, and the lines of the rows they are under. */
export const indent = style.create({
    by: (gutter: string, depth: number) => ({
        paddingInlineStart: `calc(${gutter} + ${String(depth)} * ${INDENT})`,
    }),
    // draw a line below each parent's indicator
    guide: {
        position: "absolute",
        insetBlock: 0,
        backgroundImage: {
            default: `linear-gradient(to right, ${color.border} ${stroke.border}, transparent ${stroke.border})`,
            ":dir(rtl)": `linear-gradient(to left, ${color.border} ${stroke.border}, transparent ${stroke.border})`,
        },
        backgroundPosition: { default: "left", ":dir(rtl)": "right" },
        backgroundSize: `${INDENT} 100%`,
        pointerEvents: "none",
    },
    guideAt: (gutter: string, depth: number) => ({
        insetInlineStart: `calc(${gutter} + ${INDENT} / 2)`,
        width: `calc(${String(depth)} * ${INDENT})`,
    }),
});

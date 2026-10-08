import * as style from "@destack/style";

import { palette } from "../../palette.stylex";

/** The plates the figures set things on: solid cream for what you own, dashed for what you rent. */
export const plate = style.create({
    /** A solid cream plate with a hairline rim. */
    plate: {
        backgroundColor: palette.cream,
        borderColor: `color-mix(in srgb, ${palette.ink} 30%, transparent)`,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: palette.ink,
        fontSize: "0.75rem",
        lineHeight: 1,
        paddingBlock: "0.3125rem",
        paddingInline: "0.4375rem",
        whiteSpace: "nowrap",
    },

    /** A dashed, empty plate for what you rent. */
    closed: {
        backgroundColor: "transparent",
        borderColor: "currentColor",
        borderStyle: "dashed",
        boxShadow: "none",
        color: "inherit",
        cursor: "not-allowed",
    },
});

import * as style from "@destack/style";
import { color, motion } from "@destack/theme/tokens.stylex";

/** The look of everything pressable on the site: a hard ink shadow beneath it that presses in. */
export const press = style.create({
    key: {
        boxShadow: { default: `2px 2px 0 ${color.foreground}`, ":active": "none" },
        transform: { default: "none", ":active": "translate(2px, 2px)" },
        transitionDuration: motion.durationShort,
        transitionProperty: "background-color, box-shadow, color, transform",
        transitionTimingFunction: motion.easingStandard,
    },
});

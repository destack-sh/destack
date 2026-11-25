import caretLeftIcon from "@destack/icon/phosphor/caret-left";
import caretRightIcon from "@destack/icon/phosphor/caret-right";
import * as style from "@destack/style";
import { motion } from "@destack/theme/tokens.stylex";
import { type JSX, useLocale } from "@destack/view";
import { Icon } from "../icon/index.ts";

/** The styles of an indicator, which turns down from the reading direction once open. */
export const indicator = style.create({
    base: {
        display: "inline-flex",
        flexShrink: 0,
        transitionProperty: "rotate",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    open: {
        rotate: { default: "90deg", ":dir(rtl)": "-90deg" },
    },
});

/** Render the caret that points along the reading direction. */
export function DisclosureCaret(): JSX.Element {
    const locale = useLocale();

    return <Icon icon={locale.direction === "rtl" ? caretLeftIcon : caretRightIcon} />;
}

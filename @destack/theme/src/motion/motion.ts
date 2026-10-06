import * as style from "@destack/style";
import { motion } from "../token/tokens.stylex.ts";

/** The scale an element enters from and exits to. */
const HIDDEN_SCALE = "scale(0.96)";

/** Transition presets for elements entering and leaving, discrete properties included. */
export const transition = style.create({
    /** Fade and grow in from the element's first render, display and top layer included. */
    enter: {
        opacity: { default: 1, "@starting-style": 0 },
        transform: { default: "none", "@starting-style": HIDDEN_SCALE },
        transitionProperty: "opacity, transform, display, overlay",
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingEmphasised,
        transitionBehavior: "allow-discrete",
    },

    /** Fade and shrink out before the element leaves display and the top layer. */
    exit: {
        opacity: 0,
        transform: HIDDEN_SCALE,
        transitionProperty: "opacity, transform, display, overlay",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        transitionBehavior: "allow-discrete",
    },
});

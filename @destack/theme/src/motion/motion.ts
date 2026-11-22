import * as style from "@destack/style";
import { motion } from "../token/tokens.stylex.ts";

/** The scale an overlay grows from as it enters and shrinks to as it leaves, never from nothing. */
const HIDDEN_SCALE = "scale(0.96)";

/** The fade an overlay keeps when reduced motion drops its movement, after Emil Kowalski. */
const REDUCED_FADE = "150ms";

/** The properties an overlay moves through, in the order of each preset's durations. */
const PROPERTIES = "opacity, transform, display, overlay";

/** A popover's fade: its entrance, and at least the reduced fade. */
const SHORT_FADE = `max(${motion.durationShort}, ${REDUCED_FADE})`;

/** A popover's fade leaving: two thirds of its entrance, as a dismissal is quick. */
const SHORT_FADE_EXIT = `calc(${SHORT_FADE} * 2 / 3)`;

/** A dialog's fade: its entrance, and at least the reduced fade. */
const MEDIUM_FADE = `max(${motion.durationMedium}, ${REDUCED_FADE})`;

/** A dialog's fade leaving: two thirds of its entrance. */
const MEDIUM_FADE_EXIT = `calc(${MEDIUM_FADE} * 2 / 3)`;

/** The durations of a popover entering, for each of the properties. */
const SHORT = `${SHORT_FADE}, ${motion.durationShort}, ${SHORT_FADE}, ${SHORT_FADE}`;

/** The durations of a popover leaving. */
const SHORT_EXIT = `${SHORT_FADE_EXIT}, calc(${motion.durationShort} * 2 / 3), ${SHORT_FADE_EXIT}, ${SHORT_FADE_EXIT}`;

/** The durations of a dialog entering, which moves for longer than a popover. */
const MEDIUM = `${MEDIUM_FADE}, ${motion.durationMedium}, ${MEDIUM_FADE}, ${MEDIUM_FADE}`;

/** The durations of a dialog leaving. */
const MEDIUM_EXIT = `${MEDIUM_FADE_EXIT}, calc(${motion.durationMedium} * 2 / 3), ${MEDIUM_FADE_EXIT}, ${MEDIUM_FADE_EXIT}`;

/** A notice's movement: Sonner's slower slide, four thirds of a dialog's. */
const NOTICE_MOVE = `calc(${motion.durationMedium} * 4 / 3)`;

/** A notice's fade: its slide, and at least the reduced fade. */
const NOTICE_FADE = `max(${NOTICE_MOVE}, ${REDUCED_FADE})`;

/** A notice's movement leaving: two thirds of a dialog's entrance, as Sonner's exit. */
const NOTICE_MOVE_EXIT = `calc(${motion.durationMedium} * 2 / 3)`;

/** A notice's fade leaving: its slide out, and at least two thirds of the reduced fade. */
const NOTICE_FADE_EXIT = `max(${NOTICE_MOVE_EXIT}, calc(${REDUCED_FADE} * 2 / 3))`;

/** The fade and scale of overlays entering and leaving the top layer, after Emil Kowalski. */
export const transition = style.create({
    /** A popover over the page, growing from the side it is anchored to. */
    popover: {
        opacity: { default: 0, ":popover-open": { default: 1, "@starting-style": 0 } },
        transform: {
            default: HIDDEN_SCALE,
            ":popover-open": { default: "none", "@starting-style": HIDDEN_SCALE },
        },
        transitionProperty: PROPERTIES,
        transitionDuration: { default: SHORT_EXIT, ":popover-open": SHORT },
        transitionTimingFunction: motion.easingEmphasised,
        transitionBehavior: "allow-discrete",
    },
    /** A popover shown as a modal, growing from the side it is anchored to. */
    modalPopover: {
        opacity: { default: 0, ":modal": { default: 1, "@starting-style": 0 } },
        transform: {
            default: HIDDEN_SCALE,
            ":modal": { default: "none", "@starting-style": HIDDEN_SCALE },
        },
        transitionProperty: PROPERTIES,
        transitionDuration: { default: SHORT_EXIT, ":modal": SHORT },
        transitionTimingFunction: motion.easingEmphasised,
        transitionBehavior: "allow-discrete",
    },
    /** A dialog, growing from the middle of the screen. */
    dialog: {
        opacity: { default: 0, ":modal": { default: 1, "@starting-style": 0 } },
        transform: {
            default: HIDDEN_SCALE,
            ":modal": { default: "none", "@starting-style": HIDDEN_SCALE },
        },
        transitionProperty: PROPERTIES,
        transitionDuration: { default: MEDIUM_EXIT, ":modal": MEDIUM },
        transitionTimingFunction: motion.easingEmphasised,
        transitionBehavior: "allow-discrete",
    },
    /** A sheet, fading as it slides in from the edge its side sets. */
    sheet: {
        opacity: { default: 0, ":modal": { default: 1, "@starting-style": 0 } },
        transitionProperty: PROPERTIES,
        transitionDuration: { default: MEDIUM_EXIT, ":modal": MEDIUM },
        transitionTimingFunction: motion.easingEmphasised,
        transitionBehavior: "allow-discrete",
    },
    /** A notice such as a toast, fading as it slides in from the edge it sits at, after Sonner. */
    notice: {
        opacity: { default: 1, "@starting-style": 0 },
        transitionProperty: "opacity, transform",
        transitionDuration: `${NOTICE_FADE}, ${NOTICE_MOVE}`,
        transitionTimingFunction: "ease",
    },
    /** A notice leaving, faster than it came. */
    noticeLeaving: {
        opacity: 0,
        transitionDuration: `${NOTICE_FADE_EXIT}, ${NOTICE_MOVE_EXIT}`,
    },
});

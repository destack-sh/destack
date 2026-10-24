import * as style from "@destack/style";
import { screen } from "./screen.stylex";

/** The frame width on desktop: 1200px, with 24px to each side on narrower windows. */
const WIDTH = "min(1200px, calc(100vw - 48px))";

/** The frame width on phones: the whole window. */
const PHONE_WIDTH = "100vw";

/** The height of the top bar, the footer and toolbar rows. */
const BAR = "56px";

/** The height of one homepage section on desktop. */
const SECTION = `clamp(720px, calc(100svh - ${BAR}), 1080px)`;

/** The height of every dictionary entry on desktop, so the sections line up. */
const ENTRY = "12rem";

/** The height of a figure's label row, with its rule. */
const LABEL = "50px";

/** The stack figure window's inner width: the frame less an inset and a border each side. */
const WINDOW_WIDTH = `calc(${WIDTH} - 50px)`;

/** The stack figure window's inner width on phones, with their narrower inset. */
const PHONE_WINDOW_WIDTH = `calc(${PHONE_WIDTH} - 42px)`;

/** The stack figure's row on desktop, after its labels and its window's strip and margins. */
const STAGE = `calc((${SECTION} - ${ENTRY} - ${LABEL} - 72px) / 6)`;

/** The stack figure's row below the desktop width, which carries only the cards. */
const NARROW_STAGE = `clamp(112px, calc(${WIDTH} / 88 * 13), 134px)`;

/** The stack figure's row on phones. */
const PHONE_STAGE = "120px";

/** The stack figure's row on the smallest phones, whose narrow tiles wrap onto more lines. */
const SMALL_PHONE_STAGE = "136px";

/** The site's twelve-column frame on desktop and four on phones, and the homepage's rhythm. */
export const frame = style.defineVars({
    /** The frame width shared by every page. */
    width: { default: WIDTH, [screen.phone]: PHONE_WIDTH, [screen.smallPhone]: PHONE_WIDTH },
    /** One of the twelve frame columns, or four on phones; also the square module height. */
    column: {
        default: `calc(${WIDTH} / 12)`,
        [screen.phone]: `calc(${PHONE_WIDTH} / 4)`,
        [screen.smallPhone]: `calc(${PHONE_WIDTH} / 4)`,
    },
    /** The height of the top bar, the footer and toolbar rows. */
    bar: BAR,
    /** The homepage stack figure's row height, a fixed share of a column. */
    row: {
        default: `calc(${WIDTH} / 12 * 0.95)`,
        [screen.phone]: "72px",
        [screen.smallPhone]: "72px",
    },
    /** The height of one homepage section on desktop. */
    section: SECTION,
    /** The height of every dictionary entry on desktop. */
    entry: ENTRY,
    /** The stack figure's row height. */
    stage: {
        default: STAGE,
        [screen.tablet]: NARROW_STAGE,
        [screen.phone]: PHONE_STAGE,
        [screen.smallPhone]: SMALL_PHONE_STAGE,
    },
    /** The breadboard hole pitch across. */
    cell: {
        default: `calc(${WINDOW_WIDTH} / 12 * 8 / 88)`,
        [screen.tablet]: `calc(${WINDOW_WIDTH} / 88)`,
        [screen.phone]: `calc(${PHONE_WINDOW_WIDTH} / 88)`,
        [screen.smallPhone]: `calc(${PHONE_WINDOW_WIDTH} / 88)`,
    },
    /** The breadboard hole pitch down: nine cells to a stage row. */
    cellRow: {
        default: `calc(${STAGE} / 9)`,
        [screen.tablet]: `calc(${NARROW_STAGE} / 9)`,
        [screen.phone]: `calc(${PHONE_STAGE} / 9)`,
        [screen.smallPhone]: `calc(${SMALL_PHONE_STAGE} / 9)`,
    },
    /** The text inset inside a lattice cell. */
    inset: { default: "24px", [screen.phone]: "20px", [screen.smallPhone]: "20px" },
});

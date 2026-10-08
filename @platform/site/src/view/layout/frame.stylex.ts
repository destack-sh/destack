import * as style from "@destack/style";

/** The frame width on desktop: 1200px, with 24px to each side on narrower windows. */
const WIDTH = "min(1200px, calc(100vw - 48px))";

/** The frame width on phones: the whole window. */
const PHONE_WIDTH = "100vw";

/** The height of the top bar, the footer and toolbar rows. */
const BAR = "56px";

/** The height of one homepage section on desktop: the screen under the top bar, between 720px and 1080px. */
const SECTION = `clamp(720px, calc(100svh - ${BAR}), 1080px)`;

/** The height of every dictionary entry on desktop, so the sections line up. */
const ENTRY = "12.5rem";

/** The stack figure's row on desktop: six rows fill the section under the entry and its 50px figure label. */
const STAGE = `calc((${SECTION} - ${ENTRY} - 50px) / 6)`;

/** The stack figure's row below the desktop width: sixteen breadboard holes, between 136px and 164px. */
const NARROW_STAGE = `clamp(136px, calc(${WIDTH} / 88 * 16), 164px)`;

/** The stack figure's row on phones. */
const PHONE_STAGE = "150px";

/** The stack figure's row on the smallest phones, whose narrow tiles wrap onto more lines. */
const SMALL_PHONE_STAGE = "168px";

/** The media query for windows below the desktop width that are wider than phones. */
const NARROW = "@media (min-width: 768px) and (max-width: 1099px)";

/** The media query for phones wider than the smallest ones. */
const PHONE = "@media (min-width: 400px) and (max-width: 767px)";

/** The media query for the smallest phones. */
const SMALL_PHONE = "@media (max-width: 399px)";

/** The site's twelve-column frame on desktop and four on phones, and the homepage's rhythm. */
export const frame = style.defineVars({
    /** The frame width shared by every page. */
    width: { default: WIDTH, [PHONE]: PHONE_WIDTH, [SMALL_PHONE]: PHONE_WIDTH },
    /** One of the twelve frame columns, or four on phones; also the square module height. */
    column: {
        default: `calc(${WIDTH} / 12)`,
        [PHONE]: `calc(${PHONE_WIDTH} / 4)`,
        [SMALL_PHONE]: `calc(${PHONE_WIDTH} / 4)`,
    },
    /** The height of the top bar, the footer and toolbar rows. */
    bar: BAR,
    /** The homepage stack figure's row height, a fixed share of a column. */
    row: {
        default: `calc(${WIDTH} / 12 * 0.95)`,
        [PHONE]: "72px",
        [SMALL_PHONE]: "72px",
    },
    /** The height of one homepage section on desktop. */
    section: SECTION,
    /** The height of every dictionary entry on desktop. */
    entry: ENTRY,
    /** The stack figure's row height. */
    stage: {
        default: STAGE,
        [NARROW]: NARROW_STAGE,
        [PHONE]: PHONE_STAGE,
        [SMALL_PHONE]: SMALL_PHONE_STAGE,
    },
    /** The breadboard hole pitch across: the stack figure's drawing is 88 cells over eight columns, or the frame below the desktop width. */
    cell: {
        default: `calc(${WIDTH} / 12 * 8 / 88)`,
        [NARROW]: `calc(${WIDTH} / 88)`,
        [PHONE]: `calc(${PHONE_WIDTH} / 88)`,
        [SMALL_PHONE]: `calc(${PHONE_WIDTH} / 88)`,
    },
    /** The breadboard hole pitch down: nine cells to a stage row. */
    cellRow: {
        default: `calc(${STAGE} / 9)`,
        [NARROW]: `calc(${NARROW_STAGE} / 9)`,
        [PHONE]: `calc(${PHONE_STAGE} / 9)`,
        [SMALL_PHONE]: `calc(${SMALL_PHONE_STAGE} / 9)`,
    },
    /** The text inset inside a lattice cell. */
    inset: { default: "24px", [PHONE]: "20px", [SMALL_PHONE]: "20px" },
});

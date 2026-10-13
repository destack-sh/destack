import * as style from "@destack/style";

/** The site's breakpoints beside Tailwind's in `media.stylex`. */
export const screen = style.defineConsts({
    /** Windows below the homepage's desktop width. */
    belowDesktop: "@media (max-width: 1099px)",
    /** Windows below the desktop width that are wider than phones. */
    tablet: "@media (min-width: 768px) and (max-width: 1099px)",
    /** Phones wider than the smallest ones. */
    phone: "@media (min-width: 400px) and (max-width: 767px)",
    /** The smallest phones. */
    smallPhone: "@media (max-width: 399px)",
    /** Windows wide enough for the reader's side column. */
    reader: "@media (width >= 60rem)",
    /** Desktop windows too short for a homepage section to hold every line of its figures. */
    short: "@media (min-width: 1100px) and (max-height: 880px)",
    /** Windows too narrow for the reader's side column. */
    belowReader: "@media (width < 60rem)",
});

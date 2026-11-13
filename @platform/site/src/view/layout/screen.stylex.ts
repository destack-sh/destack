import * as style from "@destack/style";

/** The site's ranges between the breakpoints of `media.stylex`, and its content conditions. */
export const screen = style.defineConsts({
    /** Tablets: from md up to lg. */
    tablet: "@media (48rem <= width < 64rem)",
    /** Phones wider than the smallest ones, below md. */
    phone: "@media (400px <= width < 48rem)",
    /** The smallest phones, whose narrow tiles wrap onto more lines. */
    smallPhone: "@media (width < 400px)",
    /** Desktops below xl, too narrow to set the stack legend's details beside its claims. */
    narrowDesktop: "@media (64rem <= width < 80rem)",
    /** Desktops too short for a homepage section to hold every line of its figures. */
    short: "@media (width >= 64rem) and (height <= 880px)",
});

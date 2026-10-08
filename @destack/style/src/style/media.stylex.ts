import { defineConsts } from "@stylexjs/stylex";

/** The media conditions components share, as StyleX keys: the common breakpoints and variants. */
export const media = defineConsts({
    /** A screen at least 40rem wide. */
    sm: "@media (width >= 40rem)",
    /** A screen at least 48rem wide. */
    md: "@media (width >= 48rem)",
    /** A screen at least 64rem wide. */
    lg: "@media (width >= 64rem)",
    /** A screen at least 80rem wide. */
    xl: "@media (width >= 80rem)",
    /** A screen at least 96rem wide. */
    "2xl": "@media (width >= 96rem)",

    /** A screen narrower than 40rem. */
    maxSm: "@media (width < 40rem)",
    /** A screen narrower than 48rem. */
    maxMd: "@media (width < 48rem)",
    /** A screen narrower than 64rem. */
    maxLg: "@media (width < 64rem)",
    /** A screen narrower than 80rem. */
    maxXl: "@media (width < 80rem)",
    /** A screen narrower than 96rem. */
    max2xl: "@media (width < 96rem)",

    /** A device whose primary pointer hovers, which every `:hover` style sits inside. */
    hover: "@media (hover: hover)",
    /** A device whose primary pointer is coarse, such as a finger. */
    pointerCoarse: "@media (pointer: coarse)",
    /** A person who asked for less motion. */
    motionReduce: "@media (prefers-reduced-motion: reduce)",
    /** A person who allows motion. */
    motionSafe: "@media (prefers-reduced-motion: no-preference)",
    /** A person who asked for more contrast. */
    contrastMore: "@media (prefers-contrast: more)",
    /** A printed page. */
    print: "@media print",
    /** A screen held upright. */
    portrait: "@media (orientation: portrait)",
    /** A screen held sideways. */
    landscape: "@media (orientation: landscape)",
});

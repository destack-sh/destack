import { defineConsts } from "@stylexjs/stylex";

/** The media conditions components share, as StyleX keys. */
export const media = defineConsts({
    /** A device whose primary pointer hovers, which every `:hover` style sits inside. */
    hover: "@media (hover: hover)",
});

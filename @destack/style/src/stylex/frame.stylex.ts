import { defineVars } from "@stylexjs/stylex";
import { media } from "@destack/style/media.stylex";

/** A page's frame: Material's layout grid on Tailwind's breakpoints, at most 80rem wide. */
export const frame = defineVars({
    /** The columns: four on phones, eight on tablets, twelve from desktops. */
    columns: { default: "4", [media.sm]: "8", [media.lg]: "12" },
    /** The margin beside the frame, Material's compact and medium margins. */
    margin: { default: "16px", [media.sm]: "24px" },
    /** The gap between columns, the same as the margin. */
    gutter: { default: "16px", [media.sm]: "24px" },
    /** The widest the frame grows, Tailwind's widest container below 2xl. */
    max: "80rem",
});

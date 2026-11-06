import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, font, motion } from "@destack/theme/tokens.stylex";

import { frame } from "../../layout/frame.stylex";
import { createFolio, folioNumber } from "../../layout/folio";
import { palette } from "../../palette.stylex";
import { anchorOf, type Form } from "./entry";

/** Tab the page's sections down the right margin as a dictionary's thumb index. */
export function ThumbIndex(properties: { forms: readonly Form[] }) {
    // follow the section the reader is in
    const folio = createFolio(() => true);

    return (
        <nav
            aria-label="Sections"
            {...style.attrs(styles.index, folio()?.isFooterShown === true && styles.hidden)}
        >
            {properties.forms.map((form, index) => (
                <a
                    href={`#${anchorOf(form)}`}
                    title={form.partOfSpeech}
                    {...style.attrs(styles.tab, folio()?.index === index + 1 && styles.current)}
                >
                    {folioNumber(index + 1)}
                </a>
            ))}
        </nav>
    );
}

/** The thumb index styles. */
const styles = style.create({
    index: {
        display: { default: "none", "@media (min-width: 1320px)": "grid" },
        left: `calc(50% + ${frame.width} / 2)`,
        position: "fixed",
        rowGap: "3px",
        top: "50%",
        transform: "translateY(-50%)",
        transition: `opacity ${motion.durationShort} ${motion.easingStandard}`,
        zIndex: 2,
    },
    hidden: {
        opacity: 0,
        pointerEvents: "none",
    },
    tab: {
        alignContent: "center",
        backgroundColor: palette.space,
        color: {
            default: `color-mix(in srgb, ${palette.cream} 60%, transparent)`,
            ":hover": { default: null, [media.hover]: palette.cream },
        },
        display: "grid",
        fontFamily: font.code,
        fontSize: "0.6875rem",
        fontWeight: 600,
        height: "2.75rem",
        justifyItems: "center",
        letterSpacing: "0.1em",
        textDecorationLine: "none",
        transitionDuration: motion.durationShort,
        transitionProperty: "width, background-color, color",
        transitionTimingFunction: motion.easingStandard,
        width: { default: "2.75rem", ":hover": { default: null, [media.hover]: "3.25rem" } },
    },
    current: {
        backgroundColor: color.primary,
        color: palette.space,
        width: "3.25rem",
    },
});

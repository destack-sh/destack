import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, font, motion } from "@destack/theme/tokens.stylex";

import { frame } from "../../layout/frame.stylex";
import { present } from "@destack/schema";
import { createMemo } from "@destack/view";
import { createFolio, folioNumber } from "../../layout/folio";
import { palette } from "../../palette.stylex";
import { anchorOf, type Form } from "./entry";

/** The windows wide enough for the index in the margin beside the page. */
const WIDE = "@media (min-width: 1320px)";

/** The windows whose margin holds a tab slid out to its whole form: a 1200px page and 9.5rem. */
const ROOMY = "@media (min-width: 1504px)";

/** Tab the page's sections down the right margin as a dictionary's thumb index. */
export function ThumbIndex(properties: { forms: readonly Form[] }) {
    // follow the section the reader is in
    const folio = createFolio(() => true);
    const current = createMemo(() => folio()?.index ?? 0);

    return (
        <nav aria-label="Sections" {...style.attrs(styles.index)}>
            {properties.forms.map((form, index) => (
                <a
                    href={`#${anchorOf(form)}`}
                    title={form.partOfSpeech}
                    {...style.attrs(styles.tab, current() === index + 1 && styles.current)}
                >
                    {/* the number, and the form it slides out to show */}
                    <span>{folioNumber(index + 1)}</span>
                    <span {...style.attrs(styles.form)}>{form.syllables.join("·")}</span>
                </a>
            ))}
        </nav>
    );
}

/** List the page's sections from the bottom bar below the widths the thumb index fits. */
export function ThumbMenu(properties: { forms: readonly Form[] }) {
    // follow the section the reader is in, the first before any
    let menu: HTMLDetailsElement | undefined;
    const folio = createFolio(() => true);
    const current = createMemo(() => Math.max(1, folio()?.index ?? 1));
    const form = () => present(properties.forms[current() - 1], "the current section's form");

    return (
        <details ref={menu} {...style.attrs(styles.menu)}>
            {/* the current section, opening the list of every section */}
            <summary aria-label="Sections" {...style.attrs(styles.summary)}>
                <span {...style.attrs(styles.number)}>{folioNumber(current())}</span>
                <span {...style.attrs(styles.word)}>{form().syllables.join("·")}</span>
            </summary>

            {/* every section, closing the list once one is chosen */}
            <nav {...style.attrs(styles.list)}>
                {properties.forms.map((each, index) => (
                    <a
                        href={`#${anchorOf(each)}`}
                        onClick={() => menu?.removeAttribute("open")}
                        {...style.attrs(styles.item, current() === index + 1 && styles.chosen)}
                    >
                        <span {...style.attrs(styles.number)}>{folioNumber(index + 1)}</span>
                        {each.syllables.join("·")}
                        <span {...style.attrs(styles.part)}>{each.partOfSpeech}</span>
                    </a>
                ))}
            </nav>
        </details>
    );
}

/** The thumb index styles. */
const styles = style.create({
    index: {
        display: { default: "none", [WIDE]: "grid" },
        left: `calc(50% + ${frame.width} / 2)`,
        position: "fixed",
        rowGap: "3px",
        top: "50%",
        transform: "translateY(-50%)",
        zIndex: 2,
    },
    tab: {
        alignItems: "center",
        backgroundColor: palette.space,
        color: {
            default: `color-mix(in srgb, ${palette.cream} 60%, transparent)`,
            ":hover": { default: null, [media.hover]: palette.cream },
        },
        columnGap: "1.5rem",
        display: "flex",
        fontFamily: font.code,
        fontSize: "0.6875rem",
        fontWeight: 600,
        height: "2.75rem",
        letterSpacing: "0.1em",
        overflow: "hidden",
        paddingInlineStart: "0.875rem",
        textDecorationLine: "none",
        transitionDuration: motion.durationShort,
        transitionProperty: "width, background-color, color",
        transitionTimingFunction: motion.easingStandard,
        whiteSpace: "nowrap",
        width: {
            default: "2.75rem",
            ":hover": { default: null, [media.hover]: { default: "3.25rem", [ROOMY]: "9.5rem" } },
        },
    },
    current: {
        backgroundColor: color.primary,
        color: palette.space,
        width: { default: "3.25rem", [ROOMY]: "9.5rem" },
    },
    form: {
        fontWeight: 500,
        letterSpacing: "0.02em",
    },
    menu: {
        position: "relative",
    },
    summary: {
        alignItems: "center",
        columnGap: "0.5rem",
        cursor: "pointer",
        display: "flex",
        fontFamily: font.code,
        fontSize: "0.75rem",
        listStyle: "none",
        paddingInline: "0.5rem",
        whiteSpace: "nowrap",
        "::-webkit-details-marker": { display: "none" },
    },
    number: {
        color: color.primary,
        fontWeight: 600,
    },
    word: {
        display: { default: "inline", "@media (max-width: 399px)": "none" },
    },
    list: {
        backgroundColor: color.background,
        borderColor: color.border,
        borderStyle: "solid",
        borderWidth: "1px",
        bottom: "calc(100% + 1.25rem)",
        boxShadow: `0 6px 24px color-mix(in srgb, ${color.foreground} 14%, transparent)`,
        display: "grid",
        minWidth: "18rem",
        padding: "0.375rem",
        position: "absolute",
        right: "-3rem",
    },
    item: {
        alignItems: "baseline",
        color: color.foreground,
        columnGap: "0.75rem",
        display: "grid",
        fontFamily: font.code,
        fontSize: "0.8125rem",
        gridTemplateColumns: "auto 1fr auto",
        paddingBlock: "0.625rem",
        paddingInline: "0.75rem",
        textDecorationLine: "none",
    },
    chosen: {
        backgroundColor: `color-mix(in srgb, ${color.primary} 10%, transparent)`,
    },
    part: {
        color: color.mutedForeground,
        fontSize: "0.6875rem",
        whiteSpace: "nowrap",
    },
});

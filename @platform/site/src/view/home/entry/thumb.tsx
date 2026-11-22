import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { font, motion, swatch } from "@destack/theme/tokens.stylex";

import { measure } from "../../layout/measure.stylex";
import { present } from "@destack/schema";
import { createMemo } from "@destack/view";
import { createFolio, folioNumber } from "../../layout/folio";
import { palette } from "../../palette.stylex";
import { typeScale } from "../../layout/type.stylex";
import { anchorOf, type Form } from "./entry";

/** The windows wide enough for the index in the margin beside the page. */
const WIDE = "@media (min-width: 1400px)";

/** The windows whose margin holds a tab slid out to its whole form: a 1280px page and 11rem. */
const ROOMY = "@media (min-width: 1632px)";

/** A tab's height and the gap under it, in CSS pixels: 2.75rem and 3px. */
const TAB = 44;

/** How far apart two tabs' middles sit, in CSS pixels. */
const STEP = TAB + 3;

/** The line a section's top rests on once scrolled to: the top bar, 56px as `measure.bar` sets. */
const LINE = 56;

/** Tab the page's sections down the left margin, the current tab level with its headword. */
export function ThumbIndex(properties: { forms: readonly Form[] }) {
    // follow how far the reader has scrolled through the sections, as a fraction of a section
    const folio = createFolio(() => true);
    const place = createMemo(() => {
        // wait for the page to be measured
        const placed = folio();
        if (placed === undefined) {
            return undefined;
        }

        // before the first section rests on the line, follow its headword down the page
        const { tops, lead } = placed;
        const first = present(tops[0], "the first section's top");
        if (first > LINE) {
            return { section: 0, top: first + lead - TAB / 2 };
        }

        // past it, slide one tab per section between the sections resting on the line
        const rested = tops.findLastIndex((top) => top <= LINE);
        const top = present(tops[rested], "the rested section's top");
        const next = tops[rested + 1];
        const section = rested + (next === undefined ? 0 : (LINE - top) / (next - top));

        return { section, top: LINE + lead - TAB / 2 - section * STEP };
    });
    const current = createMemo(() => Math.round(place()?.section ?? -1));

    return (
        <nav
            aria-label="Sections"
            style={
                place() === undefined
                    ? undefined
                    : { top: "0px", transform: `translateY(${place()?.top ?? 0}px)` }
            }
            {...style.attrs(styles.index)}
        >
            {properties.forms.map((form, index) => (
                <a
                    href={`#${anchorOf(form)}`}
                    title={form.partOfSpeech}
                    {...style.attrs(styles.tab, current() === index && styles.current)}
                >
                    {/* the number, and the form it slides out to show */}
                    <span>{folioNumber(index + 1)}</span>
                    <span {...style.attrs(styles.form)}>{form.syllables.join("·")}</span>
                </a>
            ))}
        </nav>
    );
}

/** The thumb index styles. */
const styles = style.create({
    index: {
        display: { default: "none", [WIDE]: "grid" },
        justifyItems: "end",
        position: "fixed",
        right: `calc(50% + ${measure.width} / 2)`,
        rowGap: "3px",
        // rest tab 01 level with the first headword before the page is measured: the top bar,
        //  the entry's padding and half the headword's line, less half a tab
        top: `calc(${LINE}px + 2.75rem + ${typeScale.display} * 0.575 - ${TAB / 2}px)`,
        zIndex: 2,
    },
    tab: {
        alignItems: "center",
        backgroundColor: palette.space,
        color: {
            default: `color-mix(in srgb, ${palette.cream} 80%, transparent)`,
            ":hover": { default: null, [media.hover]: palette.cream },
        },
        columnGap: "0.875rem",
        display: "flex",
        fontFamily: font.code,
        fontSize: "0.8125rem",
        fontWeight: 600,
        height: "2.75rem",
        letterSpacing: "0.1em",
        flexDirection: "row-reverse",
        overflow: "hidden",
        paddingInlineEnd: "0.875rem",
        textDecorationLine: "none",
        transitionDuration: motion.durationShort,
        transitionProperty: "width, background-color, color",
        transitionTimingFunction: motion.easingStandard,
        whiteSpace: "nowrap",
        width: {
            default: "2.75rem",
            ":hover": { default: null, [media.hover]: { default: "3.25rem", [ROOMY]: "11rem" } },
        },
    },
    current: {
        backgroundColor: swatch.accentSolid,
        color: palette.space,
        width: { default: "3.25rem", [ROOMY]: "11rem" },
    },
    form: {
        display: { default: "none", [ROOMY]: "inline" },
        fontFamily: font.text,
        fontSize: "0.9375rem",
        fontWeight: 600,
        letterSpacing: 0,
    },
});

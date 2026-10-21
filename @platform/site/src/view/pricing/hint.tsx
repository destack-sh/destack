import { media } from "@destack/style/media.stylex";
import { color, font, shadow, weight } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { createSignal } from "@destack/view";

import { palette } from "../palette.stylex";
import { typeScale } from "../layout/type.stylex";

/** Set a term with a dotted underline that explains it in a bubble above it on hover or focus. */
export function Hint(properties: { text: string; detail: string }) {
    // show the bubble while the term is pointed at or focused
    const [isOpen, setIsOpen] = createSignal(false);

    return (
        <span
            tabindex="0"
            onPointerEnter={() => setIsOpen(true)}
            onPointerLeave={() => setIsOpen(false)}
            onFocus={() => setIsOpen(true)}
            onBlur={() => setIsOpen(false)}
            {...style.attrs(styles.term, isOpen() && styles.termOpen)}
        >
            {properties.text}
            <span role="tooltip" {...style.attrs(styles.bubble, isOpen() && styles.open)}>
                {properties.detail}
            </span>
        </span>
    );
}

/** The hint styles. */
const styles = style.create({
    term: {
        cursor: "help",
        outline: "none",
        position: "relative",
        textDecorationColor: `color-mix(in srgb, ${color.foreground} 30%, transparent)`,
        textDecorationLine: "underline",
        textDecorationStyle: "dotted",
        textDecorationThickness: "1.5px",
        textUnderlineOffset: "0.3em",
        transition: "text-decoration-color 160ms ease",
    },
    termOpen: {
        textDecorationColor: color.primary,
    },
    bubble: {
        backgroundColor: palette.ink,
        borderRadius: "6px",
        bottom: "calc(100% + 0.625rem)",
        boxShadow: shadow.overlay,
        color: palette.cream,
        fontFamily: font.text,
        fontSize: typeScale.small,
        fontWeight: weight.regular,
        insetInlineStart: 0,
        lineHeight: 1.45,
        opacity: 0,
        paddingBlock: "0.5rem",
        paddingInline: "0.75rem",
        pointerEvents: "none",
        position: "absolute",
        textAlign: "start",
        transform: "translateY(4px)",
        transition: {
            default: "opacity 160ms ease, transform 160ms ease",
            [media.motionReduce]: "none",
        },
        whiteSpace: "normal",
        width: "max-content",
        maxWidth: "16rem",
        zIndex: 2,
    },
    open: {
        opacity: 1,
        transform: "none",
    },
});

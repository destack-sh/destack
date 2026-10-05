import { color, text } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";

import { commandEvents } from "../command/command";
import { charge } from "../effect/goo";
import { tokens } from "../style/tokens.stylex";
import { Syllables } from "./entry";

/** How hard the goo charges while the switch is hovered. */
const hoverCharge = 0.5;
/** How hard the goo charges while the switch is held down. */
const heldCharge = 1.8;

/** The switch's word for the stack as it is today. */
const stacked = ["stacked"];

/** The switch's word for the stack destacked. */
const destacked = ["de", "stacked"];

/** Switch the whole page, and every figure on it, between the stack as it is and the stack destacked. */
export function StackSwitch(properties: { isOpen: boolean }) {
    return (
        <button
            type="button"
            role="switch"
            aria-label="Destack"
            aria-checked={properties.isOpen ? "true" : "false"}
            title={properties.isOpen ? "Restack" : "Destack"}
            onClick={() => document.dispatchEvent(new CustomEvent(commandEvents.switchStack))}
            onPointerEnter={() => charge(properties.isOpen ? 0 : hoverCharge)}
            onPointerLeave={() => charge(0)}
            onPointerDown={() => {
                // tick the switch and charge the goo harder while held
                charge(properties.isOpen ? 0 : heldCharge);
            }}
            {...stylex.attrs(styles.switch)}
        >
            <b {...stylex.attrs(styles.word, !properties.isOpen && styles.active)}>
                <Syllables syllables={stacked} />
            </b>
            <span
                aria-hidden="true"
                {...stylex.attrs(styles.track, properties.isOpen && styles.trackOn)}
            >
                <span
                    {...stylex.attrs(
                        styles.knob,
                        properties.isOpen ? styles.knobOn : styles.knobNudge,
                    )}
                />
            </span>
            <b {...stylex.attrs(styles.word, properties.isOpen && styles.active)}>
                <Syllables syllables={destacked} />
            </b>
        </button>
    );
}

/** The knob's periodic nudge toward destacked. */
const nudge = stylex.keyframes({
    "0%, 80%, 100%": { translate: "0 0" },
    "86%": { translate: "0.5rem 0" },
    "91%": { translate: "0.125rem 0" },
    "95%": { translate: "0.25rem 0" },
});

/** The switch styles. */
const styles = stylex.create({
    switch: {
        alignItems: "center",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: "inherit",
        cursor: "pointer",
        display: "inline-flex",
        fontFamily: text.family,
        gap: "0.625rem",
        padding: 0,
    },
    word: {
        color: color.mutedForeground,
        fontSize: "0.875rem",
        fontWeight: 600,
        transitionDuration: "300ms",
        transitionProperty: "color",
    },
    active: {
        color: tokens.signal,
    },
    track: {
        borderColor: tokens.signal,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1.5px",
        display: "flex",
        height: "1.25rem",
        padding: "2px",
        transition: "background-color 300ms ease",
        width: "2.25rem",
    },
    trackOn: {
        backgroundColor: tokens.signal,
    },
    knob: {
        backgroundColor: tokens.signal,
        borderRadius: "50%",
        height: "0.8125rem",
        transition: "translate 450ms cubic-bezier(0.4, 0, 0.2, 1.3)",
        width: "0.8125rem",
        "@media (prefers-reduced-motion: reduce)": { transition: "none" },
    },
    knobOn: {
        backgroundColor: tokens.space,
        translate: "1rem 0",
    },
    knobNudge: {
        animationDuration: "4s",
        animationIterationCount: "infinite",
        animationName: nudge,
        "@media (prefers-reduced-motion: reduce)": { animationName: "none" },
    },
});

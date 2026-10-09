import { color, font } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { palette } from "../../palette.stylex";

import { commandEvents } from "../../layout/command";
import { charge } from "../../effect/goo";
import { Syllables } from "./entry";
import { media } from "@destack/style/media.stylex";

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
        <span {...style.attrs(styles.holder)}>
            <button
                type="button"
                role="switch"
                aria-label="Destack"
                aria-checked={properties.isOpen ? "true" : "false"}
                title={properties.isOpen ? "Restack" : "Destack"}
                onClick={() => {
                    // flip the figures' rebuilt parts in from now on and switch every figure
                    document.documentElement.dataset["switched"] = "";
                    document.dispatchEvent(new CustomEvent(commandEvents.switchStack));
                }}
                onPointerEnter={() => charge(properties.isOpen ? 0 : hoverCharge)}
                onPointerLeave={() => charge(0)}
                onPointerDown={() => {
                    // tick the switch and charge the goo harder while held
                    charge(properties.isOpen ? 0 : heldCharge);
                }}
                {...style.attrs(styles.switch)}
            >
                <b {...style.attrs(styles.word, !properties.isOpen && styles.active)}>
                    <Syllables syllables={stacked} />
                </b>
                <span
                    aria-hidden="true"
                    {...style.attrs(styles.track, properties.isOpen && styles.trackOn)}
                >
                    <span
                        {...style.attrs(
                            styles.knob,
                            properties.isOpen ? styles.knobOn : styles.knobNudge,
                        )}
                    />
                </span>
                <b {...style.attrs(styles.word, properties.isOpen && styles.active)}>
                    <Syllables syllables={destacked} />
                </b>
            </button>
        </span>
    );
}

/** The knob's periodic nudge toward destacked. */
const nudge = style.keyframes({
    "0%, 80%, 100%": { translate: "0 0" },
    "86%": { translate: "0.5rem 0" },
    "91%": { translate: "0.125rem 0" },
    "95%": { translate: "0.25rem 0" },
});

/** The switch styles. */
const styles = style.create({
    switch: {
        alignItems: "center",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: "inherit",
        cursor: "pointer",
        display: "inline-flex",
        fontFamily: font.text,
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
        color: color.primary,
    },
    track: {
        borderColor: color.primary,
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
        backgroundColor: color.primary,
    },
    knob: {
        backgroundColor: color.primary,
        borderRadius: "50%",
        height: "0.8125rem",
        transition: {
            default: "translate 450ms cubic-bezier(0.4, 0, 0.2, 1.3)",
            [media.motionReduce]: "none",
        },
        width: "0.8125rem",
    },
    knobOn: {
        backgroundColor: palette.space,
        translate: "1rem 0",
    },
    holder: {
        alignItems: "center",
        display: "inline-flex",
        position: "relative",
    },
    knobNudge: {
        animationDuration: "4s",
        animationIterationCount: "infinite",
        animationName: { default: nudge, [media.motionReduce]: "none" },
    },
});

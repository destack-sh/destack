import { color, font } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { Icon } from "@destack/icon";
import arrowUp from "@destack/icon/phosphor/arrow-up";
import list from "@destack/icon/phosphor/list";
import { Button } from "@destack/ui/button";
import { createSignal, onSettled } from "@destack/view";
import { isMotionReduced } from "@destack/view/motion";
import { createEventListener } from "@destack/view/primitives/event-listener";
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

/** Where a stack switch sits: in a figure's label, or in the phones' bottom bar. */
export type SwitchPlace = "label" | "bar";

/** Switch the whole page and every figure between the stack as it is and destacked. */
export function StackSwitch(properties: { isOpen: boolean; place: SwitchPlace }) {
    // leave phones to the bottom bar's switch
    const isInLabel = () => properties.place === "label";

    return (
        <span {...style.attrs(styles.place, isInLabel() && styles.placeLabel)}>
            <Toggle
                isOn={properties.isOpen}
                off={stacked}
                on={destacked}
                label="Destack"
                title={properties.isOpen ? "Restack" : "Destack"}
                isCharging
                onToggle={() => {
                    // flip the figures' rebuilt pieces in from now on and switch every figure
                    document.documentElement.dataset["switched"] = "";
                    document.dispatchEvent(new CustomEvent(commandEvents.switchStack));
                }}
            />
        </span>
    );
}

/** Switch between two states by their words, with a track between them. */
export function Toggle(properties: {
    isOn: boolean;
    off: readonly string[];
    on: readonly string[];
    label: string;
    title: string;
    isCharging: boolean;
    onToggle: () => void;
}) {
    // charge the goo while hovered or held, when the toggle asks for it
    const chargeTo = (amount: number) => {
        if (properties.isCharging) {
            charge(properties.isOn ? 0 : amount);
        }
    };

    return (
        <span {...style.attrs(styles.holder)}>
            <button
                type="button"
                role="switch"
                aria-label={properties.label}
                aria-checked={properties.isOn ? "true" : "false"}
                title={properties.title}
                onClick={() => properties.onToggle()}
                onPointerEnter={() => chargeTo(hoverCharge)}
                onPointerLeave={() => chargeTo(0)}
                onPointerDown={() => chargeTo(heldCharge)}
                {...style.attrs(styles.switch)}
            >
                <b {...style.attrs(styles.word, !properties.isOn && styles.active)}>
                    <Syllables syllables={properties.off} />
                </b>
                <span
                    aria-hidden="true"
                    {...style.attrs(styles.track, properties.isOn && styles.trackOn)}
                >
                    <span
                        {...style.attrs(
                            styles.knob,
                            properties.isOn ? styles.knobOn : styles.knobNudge,
                        )}
                    />
                </span>
                <b {...style.attrs(styles.word, properties.isOn && styles.active)}>
                    <Syllables syllables={properties.on} />
                </b>
            </button>
        </span>
    );
}

/** Hold the phone's bottom bar from the first figure to the footer. */
export function BottomBar(properties: { isOpen: boolean }) {
    // show the bar from the first figure label until the footer comes into view
    let bar: HTMLDivElement | undefined;
    const [isShown, setIsShown] = createSignal(false);
    const update = () => {
        // find where the figures start and where the page ends
        const first = document.querySelector("[data-figure-label]");
        const footer = document.querySelector("[data-component=SiteFooter]");
        if (first === null || footer === null) {
            throw new Error("the page lacks its figures or its footer");
        }

        // show the bar between the two
        setIsShown(
            first.getBoundingClientRect().top < window.innerHeight &&
                footer.getBoundingClientRect().top > window.innerHeight,
        );
    };
    createEventListener(() => window, "scroll", update, { passive: true });
    createEventListener(() => window, "resize", update);
    onSettled(update);

    // scroll back to the top, at once while motion is reduced
    const toTop = () => {
        const isReduced = bar !== undefined && isMotionReduced(bar);
        window.scrollTo({ top: 0, behavior: isReduced ? "instant" : "smooth" });
    };

    return (
        <div
            ref={bar}
            data-component="BottomBar"
            {...style.attrs(styles.bar, !isShown() && styles.barHidden)}
        >
            <Button variant="ghost" size="icon" aria-label="Back to top" onClick={toTop}>
                <Icon icon={arrowUp} xstyle={styles.barIcon} />
            </Button>
            <StackSwitch isOpen={properties.isOpen} place="bar" />
            <Button
                variant="ghost"
                size="icon"
                aria-label="Menu"
                onClick={() => document.dispatchEvent(new CustomEvent(commandEvents.openMenu))}
            >
                <Icon icon={list} xstyle={styles.barIcon} />
            </Button>
        </div>
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
    place: {
        display: "contents",
    },
    placeLabel: {
        display: { default: "contents", [media.maxMd]: "none" },
    },
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
    bar: {
        alignItems: "center",
        backgroundColor: color.background,
        borderColor: color.border,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        bottom: "calc(0.75rem + env(safe-area-inset-bottom))",
        boxShadow: `0 6px 24px color-mix(in srgb, ${color.foreground} 14%, transparent)`,
        boxSizing: "border-box",
        display: { default: "none", [media.maxMd]: "grid" },
        gridTemplateColumns: "auto minmax(0, 1fr) auto",
        height: "3.5rem",
        insetInline: "1rem",
        justifyItems: "center",
        marginInline: "auto",
        maxWidth: "28rem",
        paddingInline: "0.5rem",
        position: "fixed",
        transition: {
            default: "opacity 200ms ease, translate 200ms ease",
            [media.motionReduce]: "none",
        },
        zIndex: 3,
    },
    barHidden: {
        opacity: 0,
        pointerEvents: "none",
        translate: "0 1rem",
    },
    barIcon: {
        height: "1.25rem",
        width: "1.25rem",
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

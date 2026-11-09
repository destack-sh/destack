import { color, font, motion } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { Icon } from "@destack/icon";
import arrowUp from "@destack/icon/phosphor/arrow-up";
import list from "@destack/icon/phosphor/list";
import { Button } from "@destack/ui/button";
import { createEffect, createSignal, onSettled } from "@destack/view";
import { createVisibilityObserver } from "@destack/view/primitives/intersection-observer";
import { Timeout } from "@destack/view/primitives/timer";
import { isMotionReduced } from "@destack/view/motion";
import { createEventListener } from "@destack/view/primitives/event-listener";
import { palette } from "../../palette.stylex";

import { commandEvents } from "../../layout/command";
import { type Form, Syllables } from "./entry";
import { ThumbMenu } from "./thumb";
import { media } from "@destack/style/media.stylex";
import { typeScale } from "../../layout/type.stylex";
import { press } from "../../layout/press";

/** The switch's word for the stack as it is today. */
const stacked = ["stacked"];

/** The switch's word for the stack destacked. */
const destacked = ["de", "stacked"];

/** Where a stack switch sits: in a figure's label, or in the phones' bottom bar. */
export type SwitchPlace = "label" | "bar";

/** The milliseconds a hinted switch rests before a countdown: time to read its figure. */
const HINT_DELAY = 10_000;

/** Switch the whole page and every figure between the stack as it is and destacked. */
export function StackSwitch(properties: {
    isOpen: boolean;
    place: SwitchPlace;
    isHinted?: boolean;
}) {
    // leave phones to the bottom bar's switch
    const isInLabel = () => properties.place === "label";

    return (
        <span {...style.attrs(styles.place, isInLabel() && styles.placeLabel)}>
            <Toggle
                isHinted={properties.isHinted === true}
                isOn={properties.isOpen}
                off={stacked}
                on={destacked}
                label="Destack"
                title={properties.isOpen ? "Restack" : "Destack"}
                onToggle={() => {
                    // flip the figures' rebuilt pieces in from now on and switch every figure
                    document.documentElement.dataset["switched"] = "";
                    document.dispatchEvent(new CustomEvent(commandEvents.switchStack));
                }}
            />
        </span>
    );
}

/** Switch between two states as a title block: two ruled cells, the current one filled. */
export function Toggle(properties: {
    isHinted?: boolean;
    isOn: boolean;
    off: readonly string[];
    on: readonly string[];
    label: string;
    title: string;
    onToggle: () => void;
}) {
    // count down to flipping itself while it shows untouched: rest, then fill the other cell, then flip
    const [button, setButton] = createSignal<HTMLElement>();
    const isShown = createVisibilityObserver(button, { initialValue: false });
    const [isCounting, setIsCounting] = createSignal(false);
    const [isTouched, setIsTouched] = createSignal(false);
    const rest = new Timeout();
    createEffect(
        () => ({
            isDue: properties.isHinted === true && isShown() && !isTouched(),
            isOn: properties.isOn,
        }),
        ({ isDue }) => {
            // rest before each countdown, and give up once it is touched or leaves
            setIsCounting(false);
            if (isDue) {
                rest.set(() => setIsCounting(true), HINT_DELAY);
            } else {
                rest.clear();
            }
        },
    );

    // stop for good once the reader touches it, or switches the page some other way
    let flipping = properties.isOn;
    const touch = () => {
        setIsTouched(true);
        setIsCounting(false);
    };
    createEffect(
        () => properties.isOn,
        (isOn) => {
            if (flipping !== isOn) {
                touch();
            }
        },
    );

    // flip once a countdown's fill completes
    const flip = () => {
        flipping = !properties.isOn;
        properties.onToggle();
    };

    return (
        <span {...style.attrs(styles.holder)}>
            <button
                type="button"
                role="switch"
                aria-label={properties.label}
                aria-checked={properties.isOn ? "true" : "false"}
                title={properties.title}
                ref={setButton}
                onPointerEnter={touch}
                onClick={() => {
                    touch();
                    properties.onToggle();
                }}
                {...style.attrs(press.key, styles.switch)}
            >
                <b
                    onAnimationEnd={flip}
                    {...style.attrs(
                        styles.word,
                        properties.isOn ? styles.idle : styles.active,
                        properties.isOn && isCounting() && styles.hint,
                    )}
                >
                    <Syllables syllables={properties.off} />
                </b>
                <b
                    onAnimationEnd={flip}
                    {...style.attrs(
                        styles.word,
                        styles.second,
                        properties.isOn ? styles.active : styles.idle,
                        !properties.isOn && isCounting() && styles.hint,
                    )}
                >
                    <Syllables syllables={properties.on} />
                </b>
            </button>
        </span>
    );
}

/** Hold the bottom bar below the thumb index's widths, from the first figure to the footer. */
export function BottomBar(properties: { isOpen: boolean; forms: readonly Form[] }) {
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
            <ThumbMenu forms={properties.forms} />
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

/** A fill sweeping across a cell, as a countdown to switching to it. */
const sweep = style.keyframes({
    "0%": { backgroundSize: "0% 100%" },
    "100%": { backgroundSize: "100% 100%" },
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
        alignItems: "stretch",
        backgroundColor: color.background,
        borderColor: color.foreground,
        borderStyle: "solid",
        borderWidth: "1px",
        color: "inherit",
        cursor: "pointer",
        display: "inline-flex",
        fontFamily: font.code,
        padding: 0,
        position: "relative",
        zIndex: 1,
    },
    word: {
        backgroundImage: `linear-gradient(color-mix(in srgb, ${color.primary} 38%, transparent), color-mix(in srgb, ${color.primary} 38%, transparent))`,
        backgroundPosition: "left",
        backgroundRepeat: "no-repeat",
        backgroundSize: "0% 100%",
        color: color.mutedForeground,
        fontSize: typeScale.label,
        fontWeight: 600,
        letterSpacing: "0.1em",
        paddingBlock: "0.4375rem",
        paddingInline: "0.75rem",
        textTransform: "uppercase",
        transitionDuration: motion.durationShort,
        transitionProperty: "background-color, background-size, color",
        transitionTimingFunction: motion.easingStandard,
    },
    second: {
        borderInlineStartColor: color.foreground,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "1px",
    },
    active: {
        backgroundColor: color.primary,
        color: palette.space,
    },
    idle: {
        backgroundSize: {
            default: "0% 100%",
            ":hover": { default: null, [media.hover]: "50% 100%" },
        },
    },
    hint: {
        animationDuration: "4s",
        animationName: { default: sweep, [media.motionReduce]: "none" },
        animationTimingFunction: "linear",
    },
    bar: {
        alignItems: "center",
        backgroundColor: color.background,
        borderColor: color.border,
        borderRadius: 0,
        borderStyle: "solid",
        borderWidth: "1px",
        bottom: "calc(0.75rem + env(safe-area-inset-bottom))",
        boxShadow: `0 6px 24px color-mix(in srgb, ${color.foreground} 14%, transparent)`,
        boxSizing: "border-box",
        display: { default: "none", "@media (max-width: 1319px)": "grid" },
        gridTemplateColumns: "auto minmax(0, 1fr) auto auto",
        height: "3.5rem",
        insetInline: "1rem",
        justifyItems: "center",
        marginInline: "auto",
        maxWidth: "28rem",
        paddingInline: "0.5rem",
        position: "fixed",
        transition: `opacity ${motion.durationShort} ${motion.easingStandard}, translate ${motion.durationShort} ${motion.easingStandard}`,
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
});

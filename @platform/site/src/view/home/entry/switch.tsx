import { color, font, motion } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { Icon } from "@destack/icon";
import arrowUp from "@destack/icon/phosphor/arrow-up";
import list from "@destack/icon/phosphor/list";
import { Button } from "@destack/ui/button";
import { createEffect, createMemo, createSignal, onSettled } from "@destack/view";
import { createVisibilityObserver } from "@destack/view/primitives/intersection-observer";
import { Timeout } from "@destack/view/primitives/timer";
import { isMotionReduced } from "@destack/view/motion";
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

/** The milliseconds a hinted switch rests before it leans over: time to read the stacked figure. */
const HINT_DELAY = 8000;

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
    // flip itself once while it shows untouched and stacked: rest, then lean toward the other side, then flip
    const [button, setButton] = createSignal<HTMLElement>();
    const isShown = createVisibilityObserver(button, { initialValue: false });
    const [isCounting, setIsCounting] = createSignal(false);
    const [isTouched, setIsTouched] = createSignal(false);
    const rest = new Timeout();
    createEffect(
        () => ({
            isDue: properties.isHinted === true && isShown() && !isTouched() && !properties.isOn,
            isOn: properties.isOn,
        }),
        ({ isDue }) => {
            // rest before the lean, and give up once it is touched, leaves, or has flipped
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

    // flip once the lean completes
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
                {/* the thumb, on the current side, leaning toward the other while it counts down */}
                <span
                    aria-hidden="true"
                    onAnimationEnd={flip}
                    {...style.attrs(
                        styles.thumb,
                        properties.isOn && styles.thumbOn,
                        isCounting() && styles.lean,
                    )}
                />

                {/* the two sides, the current one dark on the thumb */}
                <b {...style.attrs(styles.word, !properties.isOn && styles.current)}>
                    <Syllables syllables={properties.off} />
                </b>
                <b {...style.attrs(styles.word, properties.isOn && styles.current)}>
                    <Syllables syllables={properties.on} />
                </b>
            </button>
        </span>
    );
}

/** Hold the bottom bar below the thumb index's widths, from the first figure to the footer. */
export function BottomBar(properties: { isOpen: boolean; forms: readonly Form[] }) {
    // find where the figures start and where the page ends, once the page is drawn
    let bar: HTMLDivElement | undefined;
    const [first, setFirst] = createSignal<Element>();
    const [footer, setFooter] = createSignal<Element>();
    onSettled(() => {
        // read the first figure label and the footer, both drawn by now
        const label = document.querySelector("[data-figure-label]");
        const end = document.querySelector("[data-component=SiteFooter]");
        if (label === null || end === null) {
            throw new Error("the page lacks its figures or its footer");
        }
        setFirst(label);
        setFooter(end);
    });

    // show the bar once the first figure has risen into view, until the footer has
    const isFirstRisen = createVisibilityObserver(first, { initialValue: false }, risen);
    const isFooterRisen = createVisibilityObserver(footer, { initialValue: false }, risen);
    const isShown = createMemo(() => isFirstRisen() && !isFooterRisen());

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

/** A thumb stretching toward the other side, slowly at first, before it snaps across. */
const stretch = style.keyframes({
    "0%": { scale: "1 1" },
    "100%": { scale: "1.14 1" },
});

/** Decide whether an element has risen into view or past the screen's top, from its entry. */
function risen(entry: IntersectionObserverEntry): boolean {
    return entry.isIntersecting || entry.boundingClientRect.top < 0;
}

/** The switch styles. */
const styles = style.create({
    place: {
        display: "contents",
    },
    placeLabel: {
        display: { default: "contents", [media.maxMd]: "none" },
    },
    switch: {
        backgroundColor: color.background,
        borderColor: color.foreground,
        borderStyle: "solid",
        borderWidth: "1px",
        color: "inherit",
        cursor: "pointer",
        display: "inline-grid",
        fontFamily: font.code,
        gridTemplateColumns: "1fr 1fr",
        padding: 0,
        position: "relative",
        zIndex: 1,
    },
    thumb: {
        backgroundColor: color.primary,
        insetBlock: 0,
        insetInlineStart: 0,
        position: "absolute",
        transformOrigin: "left",
        transitionDuration: motion.durationMedium,
        transitionProperty: "translate",
        transitionTimingFunction: motion.easingEmphasised,
        translate: "0 0",
        width: "50%",
    },
    thumbOn: {
        transformOrigin: "right",
        translate: "100% 0",
    },
    lean: {
        animationDuration: "1.6s",
        animationFillMode: "forwards",
        animationName: { default: stretch, [media.motionReduce]: "none" },
        animationTimingFunction: "cubic-bezier(0.5, 0, 0.9, 0.6)",
    },
    word: {
        color: {
            default: color.mutedForeground,
            ":hover": { default: null, [media.hover]: color.foreground },
        },
        fontSize: typeScale.label,
        fontWeight: 600,
        letterSpacing: "0.1em",
        paddingBlock: "0.4375rem",
        paddingInline: "0.75rem",
        position: "relative",
        textAlign: "center",
        textTransform: "uppercase",
        transitionDuration: motion.durationMedium,
        transitionProperty: "color",
        transitionTimingFunction: motion.easingStandard,
    },
    current: {
        color: palette.space,
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
        display: { default: "none", "@media (max-width: 1399px)": "grid" },
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

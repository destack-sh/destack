import { color, font } from "@destack/theme/tokens.stylex";
import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuRadioGroup,
    DropdownMenuRadioItem,
    DropdownMenuTrigger,
} from "@destack/ui/dropdown-menu";
import { present } from "@destack/schema";
import { createSignal, For, type JSX, Portal, Show } from "@destack/view";
import { createEventListener } from "@destack/view/primitives/event-listener";

import { Favicon } from "./glyph";
import { ItemBadge } from "./item";

/** How long the tray stays open after the pointer leaves, in milliseconds, to forgive a slip. */
const LEAVE_TIME = 180;

/** How long the tray takes to fold back into the row's mark, in milliseconds. */
const FOLD_TIME = 180;

/** How much later each tile further from the mark slides out, in milliseconds. */
const TILE_STAGGER = 30;

/** The space between the tray's tiles, in pixels. */
const TILE_GAP = 6;

/** The side of the row's mark the tray grows to. */
type Side = "left" | "right";

/** The space around the tray's tiles in pixels, so the shown tile lands on the row's mark. */
const TRAY_PADDING = 4;

/** The mark of an option: a shape from the icon set in its tint, or the vendor's own picture. */
export type OptionMark = { icon: string; tint: string } | { logo: string };

/** One option a picker turns through: a product's name and mark. */
export type Option = { name: string; mark: OptionMark };

/**
 * Draw a small picker sized to sit in a figure's label row, like the dial.
 *
 * The shown option turns on its own while a hairline fills, and its caret opens every option.
 */
export function Picker(properties: {
    label: string;
    options: readonly Option[];
    shown: string;
    progress: number | undefined;
    onChoose: (name: string) => void;
}) {
    return (
        <span data-component="Picker" {...style.attrs(styles.picker)}>
            <span aria-hidden="true">{properties.label}</span>
            <DropdownMenu>
                <DropdownMenuTrigger
                    variant="ghost"
                    size="sm"
                    aria-label={`${properties.label}: ${properties.shown}`}
                    xstyle={styles.trigger}
                >
                    <span {...style.attrs(styles.names)}>
                        <For each={properties.options}>
                            {(option) => (
                                <Name
                                    option={option}
                                    isShown={option.name === properties.shown}
                                    isMarked
                                    isFitted={false}
                                />
                            )}
                        </For>
                    </span>
                    <svg aria-hidden="true" viewBox="0 0 24 24" {...style.attrs(styles.caret)}>
                        <path d="m6 9 6 6 6-6" />
                    </svg>
                    <Hairline progress={properties.progress} />
                </DropdownMenuTrigger>
                <OptionMenu
                    options={properties.options}
                    shown={properties.shown}
                    onChoose={properties.onChoose}
                />
            </DropdownMenu>
        </span>
    );
}

/**
 * Wrap a row's mark and name in a quiet picker: pointing at either opens a tray of every option.
 *
 * The tray grows to the left out of the row's mark, its tiles drawn like the mark itself.
 */
export function InlinePicker(properties: {
    label: string;
    options: readonly Option[];
    shown: string;
    onChoose: (name: string) => void;
    children: JSX.Element;
}) {
    // place the tray while it is on screen, and show it a frame later so it grows out of the mark
    const [anchor, setAnchor] = createSignal<DOMRect | undefined>(undefined);
    const [isShown, setIsShown] = createSignal(false);

    // order the tiles as the tray opens, and keep that order while it stays open
    const [lineup, setLineup] = createSignal<readonly Option[]>([]);

    // grow the tray to the left of the mark, or to its right where the window has no room left of it
    const [side, setSide] = createSignal<Side>("left");
    let wrapper: HTMLSpanElement | undefined;
    let tray: HTMLSpanElement | undefined;
    let closing: ReturnType<typeof setTimeout> | undefined;

    // open the tray on the row's mark while pointed at, or from the caret on a touch
    const open = () => {
        // stop any fold, and on a fresh open choose the side and freeze the order
        clearTimeout(closing);
        const rect = wrapper?.firstElementChild?.getBoundingClientRect();
        if (anchor() === undefined && rect !== undefined) {
            const count = properties.options.length;
            const width = count * rect.width + (count - 1) * TILE_GAP + 2 * TRAY_PADDING;
            const next = rect.right - width < TRAY_PADDING ? "right" : "left";
            setSide(next);
            setLineup(lineupOf(properties.options, properties.shown, next));
        }

        // place the tray on the mark, and grow it a frame later
        setAnchor(rect);
        requestAnimationFrame(() => requestAnimationFrame(() => setIsShown(true)));
    };

    // fold the tray back into the mark, then take it off screen
    const close = () => {
        clearTimeout(closing);
        setIsShown(false);
        closing = setTimeout(() => setAnchor(undefined), FOLD_TIME);
    };
    const leave = () => {
        clearTimeout(closing);
        closing = setTimeout(close, LEAVE_TIME);
    };
    const isInside = (target: EventTarget | null) =>
        target instanceof Node &&
        (wrapper?.contains(target) === true || tray?.contains(target) === true);

    // close it on a press elsewhere, or once the page scrolls under it
    createEventListener(
        () => (anchor() === undefined ? undefined : document),
        "pointerdown",
        (event) => {
            if (!isInside(event.target)) {
                close();
            }
        },
    );
    createEventListener(() => (anchor() === undefined ? undefined : window), "scroll", close, {
        capture: true,
        passive: true,
    });

    return (
        <span
            ref={wrapper}
            data-component="InlinePicker"
            onPointerEnter={(event) => {
                if (event.pointerType === "mouse") {
                    open();
                }
            }}
            onPointerLeave={(event) => {
                if (event.pointerType === "mouse") {
                    leave();
                }
            }}
            onClick={(event) => {
                if (event.target instanceof Element && event.target.closest("[data-caret]")) {
                    event.stopPropagation();
                    open();
                }
            }}
            {...style.attrs(styles.inline)}
        >
            {properties.children}
            <Show when={anchor()}>
                {(rect) => (
                    <Portal>
                        <span
                            ref={tray}
                            role="radiogroup"
                            aria-label={properties.label}
                            onPointerEnter={open}
                            onPointerLeave={leave}
                            onKeyDown={(event) => {
                                if (event.key === "Escape") {
                                    close();
                                }
                            }}
                            style={
                                side() === "left"
                                    ? {
                                          top: `${rect().top - TRAY_PADDING}px`,
                                          right: `${document.documentElement.clientWidth - rect().right - TRAY_PADDING}px`,
                                      }
                                    : {
                                          top: `${rect().top - TRAY_PADDING}px`,
                                          left: `${rect().left - TRAY_PADDING}px`,
                                      }
                            }
                            {...style.attrs(
                                styles.tray,
                                side() === "right" && styles.trayRight,
                                isShown() && styles.trayShown,
                            )}
                        >
                            <For each={lineup()}>
                                {(option, index) => (
                                    <TrayOption
                                        option={option}
                                        isChosen={option.name === properties.shown}
                                        isShown={isShown()}
                                        side={side()}
                                        delay={
                                            (side() === "left"
                                                ? lineup().length - 1 - index()
                                                : index()) * TILE_STAGGER
                                        }
                                        onChoose={properties.onChoose}
                                    />
                                )}
                            </For>
                        </span>
                    </Portal>
                )}
            </Show>
        </span>
    );
}

/** Return the options in catalog order, the shown one nearest the mark so its tile sits on it. */
function lineupOf(options: readonly Option[], shown: string, side: Side): readonly Option[] {
    const others = options.filter((option) => option.name !== shown);
    const shownOption = present(
        options.find((option) => option.name === shown),
        "shown option",
    );

    return side === "left" ? [...others, shownOption] : [shownOption, ...others];
}

/** Draw the shown option's name, rolling to the next as it changes, and the caret that opens it. */
export function PickerName(properties: { options: readonly Option[]; shown: string }) {
    return (
        <span {...style.attrs(styles.pickerName)}>
            <span {...style.attrs(styles.names)}>
                <For each={properties.options}>
                    {(option) => (
                        <Name
                            option={option}
                            isShown={option.name === properties.shown}
                            isMarked={false}
                            isFitted
                        />
                    )}
                </For>
            </span>
            <svg
                data-caret
                aria-hidden="true"
                viewBox="0 0 24 24"
                {...style.attrs(styles.caret, styles.inlineCaret)}
            >
                <path d="m6 9 6 6 6-6" />
            </svg>
        </span>
    );
}

/** Draw one option of the tray as a tile like the row's mark, its name as its title. */
function TrayOption(properties: {
    option: Option;
    isChosen: boolean;
    isShown: boolean;
    side: Side;
    delay: number;
    onChoose: (name: string) => void;
}) {
    return (
        <span
            style={{ "transition-delay": `${properties.isShown ? properties.delay : 0}ms` }}
            {...style.attrs(
                styles.tile,
                properties.side === "right" && styles.tileRight,
                properties.isShown && styles.tileShown,
            )}
        >
            <button
                type="button"
                role="radio"
                title={properties.option.name}
                aria-label={properties.option.name}
                aria-checked={properties.isChosen ? "true" : "false"}
                onClick={(event) => {
                    event.stopPropagation();
                    properties.onChoose(properties.option.name);
                }}
                {...style.attrs(styles.trayOption, properties.isChosen && styles.trayChosen)}
            >
                <ItemBadge mark={properties.option.mark} />
            </button>
        </span>
    );
}

/** Draw the menu of every option, the shown one checked. */
function OptionMenu(properties: {
    options: readonly Option[];
    shown: string;
    onChoose: (name: string) => void;
}) {
    return (
        <DropdownMenuContent align="start" xstyle={styles.menu}>
            <DropdownMenuRadioGroup value={properties.shown} onValueChange={properties.onChoose}>
                <For each={properties.options}>
                    {(option) => (
                        <DropdownMenuRadioItem value={option.name} xstyle={styles.item}>
                            <Mark mark={option.mark} size={12} />
                            {option.name}
                        </DropdownMenuRadioItem>
                    )}
                </For>
            </DropdownMenuRadioGroup>
        </DropdownMenuContent>
    );
}

/** Draw an option's mark at a size in pixels. */
function Mark(properties: { mark: OptionMark; size: number }) {
    return (
        <>
            {"icon" in properties.mark ? (
                <Favicon
                    icon={properties.mark.icon}
                    tint={properties.mark.tint}
                    size={properties.size}
                />
            ) : (
                <img
                    alt=""
                    src={`/logos/${properties.mark.logo}`}
                    style={{ height: `${properties.size}px`, width: `${properties.size}px` }}
                    {...style.attrs(styles.logo)}
                />
            )}
        </>
    );
}

/**
 * Draw one option's mark and name, rolled into place while shown and out of it otherwise.
 *
 * A fitted picker sizes to the shown name alone, the others rolling over it out of the layout.
 */
function Name(properties: {
    option: Option;
    isShown: boolean;
    isMarked: boolean;
    isFitted: boolean;
}) {
    return (
        <span
            aria-hidden={properties.isShown ? undefined : "true"}
            {...style.attrs(
                styles.name,
                properties.isShown && styles.nameShown,
                properties.isFitted && !properties.isShown && styles.nameAway,
            )}
        >
            {properties.isMarked ? <Mark mark={properties.option.mark} size={11} /> : undefined}
            {properties.option.name}
        </span>
    );
}

/** Draw the hairline under the shown option, filled as far as its turn has run. */
function Hairline(properties: { progress: number | undefined }) {
    return (
        <span
            aria-hidden="true"
            style={{ transform: `scaleX(${properties.progress ?? 0})` }}
            {...style.attrs(styles.hairline, properties.progress === undefined && styles.still)}
        />
    );
}

/** The picker styles. */
const styles = style.create({
    picker: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "inline-flex",
        flexShrink: 0,
        fontFamily: font.code,
        fontSize: "0.6875rem",
        gap: "0.5rem",
        letterSpacing: "0.1em",
        marginBlock: "-0.25rem",
        textTransform: "uppercase",
    },
    trigger: {
        alignItems: "center",
        borderColor: color.border,
        borderRadius: "5px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: color.foreground,
        fontFamily: "inherit",
        fontSize: "0.625rem",
        gap: "0.375rem",
        height: "auto",
        letterSpacing: "0.04em",
        lineHeight: "1rem",
        overflow: "hidden",
        paddingBlock: "1px",
        paddingInline: "0.375rem",
        position: "relative",
        textTransform: "inherit",
    },
    names: {
        display: "inline-grid",
        justifyItems: "start",
        position: "relative",
    },
    name: {
        alignItems: "center",
        display: "inline-flex",
        gap: "0.375rem",
        gridArea: "1 / 1",
        opacity: 0,
        transform: "translateY(70%)",
        transitionDuration: { default: "360ms", [media.motionReduce]: "0ms" },
        transitionProperty: "opacity, transform",
        transitionTimingFunction: "cubic-bezier(0.2, 0.7, 0.3, 1)",
        whiteSpace: "nowrap",
    },
    nameShown: {
        opacity: 1,
        transform: "translateY(0)",
    },
    nameAway: {
        insetInlineStart: 0,
        position: "absolute",
    },
    caret: {
        fill: "none",
        flexShrink: 0,
        height: "10px",
        opacity: 0.6,
        stroke: "currentColor",
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 2.5,
        width: "10px",
    },
    hairline: {
        backgroundColor: color.primary,
        bottom: 0,
        height: "1px",
        insetInline: 0,
        opacity: 0.7,
        position: "absolute",
        transformOrigin: "left",
        transitionDuration: { default: "250ms", [media.motionReduce]: "0ms" },
        transitionProperty: "transform",
        transitionTimingFunction: "linear",
    },
    still: {
        opacity: 0,
    },
    inline: {
        alignItems: "center",
        alignSelf: "stretch",
        display: "flex",
        flexGrow: 1,
        gap: "inherit",
        minWidth: 0,
    },
    pickerName: {
        alignItems: "center",
        display: "inline-flex",
        gap: "0.25rem",
        minWidth: 0,
    },
    tray: {
        backgroundColor: color.background,
        borderColor: `color-mix(in srgb, ${color.foreground} 26%, transparent)`,
        borderRadius: "10px",
        borderStyle: "solid",
        borderWidth: "1.5px",
        boxShadow: `0 1px 0 color-mix(in srgb, ${color.foreground} 8%, transparent), 0 2px 6px color-mix(in srgb, ${color.foreground} 12%, transparent), 0 12px 28px -8px color-mix(in srgb, ${color.foreground} 32%, transparent)`,
        clipPath: "inset(-40px -40px -40px calc(100% - 2.75rem) round 10px)",
        display: "flex",
        gap: `${TILE_GAP}px`,
        opacity: 0,
        padding: `${TRAY_PADDING - 1.5}px`,
        position: "fixed",
        transitionDuration: { default: "180ms, 120ms", [media.motionReduce]: "0ms" },
        transitionProperty: "clip-path, opacity",
        transitionTimingFunction: "cubic-bezier(0.4, 0, 0.6, 1)",
        zIndex: 50,
    },
    trayRight: {
        clipPath: "inset(-40px calc(100% - 2.75rem) -40px -40px round 10px)",
    },
    trayShown: {
        clipPath: "inset(-40px -40px -40px -40px round 10px)",
        opacity: 1,
        transitionDuration: { default: "300ms, 90ms", [media.motionReduce]: "0ms" },
        transitionTimingFunction: "cubic-bezier(0.2, 0.9, 0.25, 1)",
    },
    tile: {
        display: "flex",
        opacity: 0,
        transform: "translateX(14px) scale(0.92)",
        transitionDuration: { default: "160ms", [media.motionReduce]: "0ms" },
        transitionProperty: "opacity, transform",
        transitionTimingFunction: "cubic-bezier(0.4, 0, 0.6, 1)",
    },
    tileRight: {
        transform: "translateX(-14px) scale(0.92)",
    },
    tileShown: {
        opacity: 1,
        transform: "none",
        transitionDuration: { default: "280ms", [media.motionReduce]: "0ms" },
        transitionTimingFunction: "cubic-bezier(0.2, 0.9, 0.25, 1)",
    },
    trayOption: {
        backgroundColor: "transparent",
        borderRadius: "6px",
        borderWidth: 0,
        cursor: "pointer",
        display: "flex",
        padding: 0,
        transform: {
            default: "none",
            ":hover": { default: null, [media.hover]: "translateY(-1px)" },
        },
        transitionDuration: { default: "140ms", [media.motionReduce]: "0ms" },
        transitionProperty: "opacity, transform",
        zoom: 1.125,
    },
    trayChosen: {
        outlineColor: color.primary,
        outlineOffset: "1px",
        outlineStyle: "solid",
        outlineWidth: "1.5px",
    },
    inlineCaret: {
        opacity: { default: 0.35, ":hover": { default: null, [media.hover]: 0.8 } },
    },
    logo: {
        borderRadius: "3px",
        flexShrink: 0,
    },
    menu: {
        fontFamily: font.code,
        fontSize: "0.6875rem",
        letterSpacing: "0.04em",
        minWidth: "9rem",
        textTransform: "uppercase",
    },
    item: {
        gap: "0.5rem",
    },
});

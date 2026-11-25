import { Icon } from "@destack/ui/icon";
import type { IconBodies } from "@destack/icon";
import checkCircle from "@destack/icon/phosphor/check-circle";
import info from "@destack/icon/phosphor/info";
import warning from "@destack/icon/phosphor/warning";
import xCircle from "@destack/icon/phosphor/x-circle";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import {
    color,
    motion,
    radius,
    shadow,
    size,
    space,
    stroke,
    weight,
    width,
    swatch,
} from "@destack/theme/tokens.stylex";
import { transition } from "@destack/theme/motion";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createEffect,
    createMemo,
    createSignal,
    For,
    type JSX,
    merge,
    onCleanup,
    onSettled,
    Show,
    untrack,
    useContext,
    useEnvironment,
    useLocale,
    handleFailing,
} from "@destack/view";
import { usePageVisibility } from "@destack/view/primitives/page-visibility";
import { createResizeObserver } from "@destack/view/primitives/resize-observer";
import { Timeout } from "@destack/view/primitives/timer";
import { Button } from "../button/index.ts";
import { Spinner } from "../spinner/index.ts";
import { Hover, Swipe } from "../interaction/index.ts";
import { LayerStack } from "../layer/index.ts";
import type { Side } from "../position/index.ts";
import { type Toast, ToastManager, type ToastType } from "./manager.ts";

/** The time a toast shows before it leaves on its own, in milliseconds, as Base UI's. */
const DURATION = 5000;

/** The time a toast with an action shows, in milliseconds, long enough to read and reach it. */
const ACTION_DURATION = 8000;

/** The most toasts a toaster shows at once, as Base UI's and Sonner's. */
const VISIBLE_TOASTS = 3;

/** The time a dismissed toast slides out before it unmounts, in milliseconds, as in Sonner. */
const LEAVING_TIME = 200;

/** How far each older toast stands out behind a newer one, in pixels, as in Sonner. */
const PEEK = 14;

/** The gap between the toasts of an open stack, in pixels, as in Sonner. */
const GAP = 14;

/** The share of its size each older toast loses behind a newer one, as in Sonner. */
const SHRINK = 0.05;

/** The toasts that stay, which the focus goes to. */
const STAYING = "[data-slot=toast]:not([data-leaving], [data-limited])";

/** The position and limits of a toaster that sets none. */
const DEFAULTS: Required<Pick<ToasterProperties, "position" | "duration" | "visibleToasts">> = {
    position: "bottom-right",
    duration: DURATION,
    visibleToasts: VISIBLE_TOASTS,
};

/** The transform a toast enters from and leaves to, off the side it is swiped away toward. */
const SLIDES: Readonly<Record<Side, string>> = {
    top: "translateY(-100%)",
    right: "translateX(100%)",
    bottom: "translateY(100%)",
    left: "translateX(-100%)",
};

/** The icon of each kind of toast that shows one. */
const ICONS: Readonly<Record<"success" | "error" | "warning" | "info", IconBodies>> = {
    success: checkCircle,
    error: xCircle,
    warning,
    info,
};

/** The toasts of the nearest toaster, null outside one. */
const ToastContext = createContext<ToastManager | null>(null);

/** Read the toasts of the nearest toaster, refusing a caller outside one. */
export function useToast(): ToastManager {
    const manager = useContext(ToastContext);
    if (manager === null) {
        throw new TypeError("a toast needs a toaster around it");
    }

    return manager;
}

/** The styles of a toaster and its toasts. */
const styles = style.create({
    toaster: {
        position: "fixed",
        inset: "auto",
        width: `min(${width.toast}, 100% - 2 * ${space[4]})`,
        overflow: "visible",
        backgroundColor: "transparent",
        color: "inherit",
        outlineStyle: "none",
    },
    height: (height: string) => ({ height }),
    layer: (layer: string) => ({ zIndex: layer }),
    place: (transform: string, entrance: string) => ({
        transform: { default: transform, "@starting-style": entrance },
    }),
    toast: {
        position: "absolute",
        insetInline: 0,
        overflow: "clip",
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[4],
        backgroundColor: color.popover,
        color: color.foreground,
        boxShadow: `0 4px 12px ${shadow.cast}`,
    },
    content: {
        display: "flex",
        // move the buttons below the text where both are too wide
        flexWrap: "wrap",
        alignItems: "center",
        justifyContent: "flex-end",
        columnGap: space[2],
        minHeight: size[3],
        padding: space[3],
        paddingInlineStart: space[4],
        transitionProperty: "opacity",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    behind: {
        opacity: 0,
    },
    // fade out behind the others, and in again once newer ones leave
    limited: {
        opacity: 0,
        pointerEvents: "none",
    },
    icon: {
        display: "inline-flex",
        alignItems: "center",
        alignSelf: "flex-start",
        flexShrink: 0,
        // center on the title's first line
        height: "1lh",
        marginBlock: space[1],
    },
    body: {
        flexGrow: 1,
        flexShrink: 1,
        flexBasis: `calc(36 * ${space[1]})`,
        minWidth: 0,
        paddingBlock: space[1],
        overflowWrap: "anywhere",
    },
    title: {
        fontWeight: weight.medium,
    },
    description: {
        maxHeight: "5lh",
        overflowY: "auto",
        color: color.mutedForeground,
    },
    buttons: {
        display: "flex",
        alignItems: "center",
        maxWidth: "100%",
        gap: space[1],
    },
    action: {
        // wrap a long label
        flexShrink: 1,
        height: "auto",
        minHeight: size[1],
        whiteSpace: "normal",
        backgroundColor: {
            default: color.foreground,
            ":hover": {
                default: null,
                [media.hover]: `color-mix(in oklab, ${color.foreground} 85%, ${color.background})`,
            },
        },
        color: color.background,
    },
    success: { color: swatch.successMark },
    error: { color: swatch.destructiveMark },
    warning: { color: swatch.warningMark },
    info: { color: swatch.infoMark },
});

/** The edge of the stack each position's newest toast stands at. */
const edges = style.create({
    top: { top: 0, transformOrigin: "bottom" },
    bottom: { bottom: 0, transformOrigin: "top" },
});

/** Read the side a swipe dismisses a toast toward, from where the toasts stack. */
function swipeOf(position: ToasterPosition): Side {
    if (position.endsWith("right")) {
        return "right";
    } else if (position.endsWith("left")) {
        return "left";
    }

    return position.startsWith("top") ? "top" : "bottom";
}

/** The corner of the viewport each position stacks toasts in. */
const positions = style.create({
    "top-left": { top: space[4], left: space[4] },
    "top-center": { top: space[4], left: "50%", transform: "translateX(-50%)" },
    "top-right": { top: space[4], right: space[4] },
    "bottom-left": { bottom: space[4], left: space[4] },
    "bottom-center": { bottom: space[4], left: "50%", transform: "translateX(-50%)" },
    "bottom-right": { bottom: space[4], right: space[4] },
});

/** The corner or edge of the viewport a toaster stacks its toasts in. */
export type ToasterPosition =
    | "top-left"
    | "top-center"
    | "top-right"
    | "bottom-left"
    | "bottom-center"
    | "bottom-right";

/** The properties of a toaster. */
export interface ToasterProperties {
    /** The corner or edge the toasts stack in, the bottom right by default. */
    readonly position?: ToasterPosition;
    /** The time each toast shows in milliseconds, 5000 by default and longer with an action. */
    readonly duration?: number;
    /** The most toasts shown at once, 3 by default. */
    readonly visibleToasts?: number;
    /** The page whose toasts it shows. */
    readonly children?: JSX.Element;
}

/** Where a toast stands in the stack. */
interface ToastPlace {
    /** Its index, the newest the first. */
    readonly index: number;
    /** How far it stands from the newest one's edge, in pixels. */
    readonly offset: number;
    /** The height it is drawn at in pixels, undefined before it is measured. */
    readonly height: number | undefined;
}

/** The toasts of a toaster as they stand: their heights, and the stack's while open or closed. */
class ToastStack {
    /** Whether the stack is open, its toasts apart, while a pointer or the focus is on it. */
    readonly isOpen: Accessor<boolean>;
    /** The ids of the toasts in view, newest first, without those sliding out. */
    readonly #ids: Accessor<readonly string[]>;
    /** The ids of every toast that is rendered. */
    readonly #rendered: Accessor<readonly string[]>;
    /** The height of each toast's content in pixels, by id. */
    readonly #heights: Accessor<ReadonlyMap<string, number>>;
    /** Replace the heights. */
    readonly #setHeights: (
        update: (heights: ReadonlyMap<string, number>) => ReadonlyMap<string, number>,
    ) => void;
    /** Set whether the pointer is on the stack. */
    readonly setHovered: (isHovered: boolean) => void;
    /** Set whether the focus is on the stack. */
    readonly setFocused: (isFocused: boolean) => void;

    /** Stand some toasts, closed at first. */
    constructor(ids: Accessor<readonly string[]>, rendered: Accessor<readonly string[]>) {
        // start closed, without heights
        const [heights, setHeights] = createSignal<ReadonlyMap<string, number>>(new Map(), {
            ownedWrite: true,
        });
        const [isHovered, setHovered] = createSignal(false, { ownedWrite: true });
        const [isFocused, setFocused] = createSignal(false, { ownedWrite: true });
        this.isOpen = createMemo(() => isHovered() || isFocused());
        this.#ids = ids;
        this.#rendered = rendered;
        this.#heights = heights;
        this.#setHeights = setHeights;
        this.setHovered = setHovered;
        this.setFocused = setFocused;
    }

    /** Record a toast's height, forgetting the heights of toasts gone since. */
    measure(id: string, height: number): void {
        const ids = new Set([...this.#rendered(), id]);
        this.#setHeights(
            (heights) =>
                new Map([...new Map(heights).set(id, height)].filter(([each]) => ids.has(each))),
        );
    }

    /** Report whether a toast is past the most in view, pushed out by newer ones. */
    isLimited(id: string): boolean {
        return !this.#ids().includes(id);
    }

    /** Read where a toast stands: its index, offset and height, past the last one if limited. */
    place(id: string): ToastPlace {
        const index = this.isLimited(id) ? this.#ids().length : this.#ids().indexOf(id);

        return { index, offset: this.#offset(index), height: this.#drawn(id) };
    }

    /** Read how far a toast stands from the newest one's edge, in pixels. */
    #offset(index: number): number {
        // peek out of a closed stack, else stand past every newer toast
        if (!this.isOpen()) {
            return index * PEEK;
        }

        return this.#ids()
            .slice(0, index)
            .reduce((offset, each) => offset + this.#heightOf(each) + GAP, 0);
    }

    /** Read the height a toast is drawn at: its own, or the newest one's behind it. */
    #drawn(id: string): number | undefined {
        const [newest] = this.#ids();

        return this.#heights().get(this.isOpen() ? id : (newest ?? id));
    }

    /** Read the stack's height, in pixels. */
    total(): number {
        // take the newest toast's height while closed, else every height and the gaps
        const ids = this.#ids();
        const heights = ids.map((id) => this.#heightOf(id));
        if (!this.isOpen()) {
            return heights[0] ?? 0;
        }

        return heights.reduce((total, each) => total + each, 0) + Math.max(0, ids.length - 1) * GAP;
    }

    /** Read a toast's height, zero before it is measured. */
    #heightOf(id: string): number {
        return this.#heights().get(id) ?? 0;
    }
}

/** Give a page toasts, drawn in a live region over it that F6 moves the focus to and back. */
export function Toaster(properties: ToasterProperties): JSX.Element {
    // share the toasts with the page
    const toaster = merge(DEFAULTS, properties);
    const locale = useLocale();
    const manager = new ToastManager();

    // tell the person of an action that failed, with the reason its error declares
    handleFailing((action, reason) => {
        // replace the toast of its last failure
        const title = locale.render(action.title);
        manager.error(locale.render(t`“${title}” did not work`), {
            id: `action-${action.package.id}/${action.name}`,
            ...(reason === undefined
                ? {}
                : { description: `${reason.charAt(0).toUpperCase()}${reason.slice(1)}` }),
        });
    });
    let region: HTMLElement | undefined;
    let stack: HTMLElement | undefined;

    // stand the newest toasts, the ones sliding out beside them
    const staying = createMemo(() =>
        manager.toasts().filter((entry) => !manager.leaving().has(entry.id)),
    );
    const inView = createMemo(() => staying().slice(0, toaster.visibleToasts));

    // render as many past them, which fade out and come back as newer ones leave
    const shown = createMemo(() => {
        const rendered = new Set(staying().slice(0, 2 * toaster.visibleToasts));

        return manager
            .toasts()
            .filter((entry) => rendered.has(entry) || manager.leaving().has(entry.id));
    });
    const standing = new ToastStack(
        () => inView().map((entry) => entry.id),
        () => shown().map((entry) => entry.id),
    );
    const hover = new Hover({
        onHoverStart: () => standing.setHovered(true),
        onHoverEnd: () => standing.setHovered(false),
    });

    // move the focus to the toasts and back with F6
    const focus = new ToasterFocus(() => region);

    // unmount each dismissed toast once it has slid out
    const { clock } = useEnvironment();
    createEffect(manager.leaving, (ids, before) => {
        for (const id of ids) {
            if (before?.has(id) !== true) {
                clock.setTimeout(() => manager.remove(id), LEAVING_TIME);
            }
        }
    });

    // show the stack in the top layer while toasts exist, inside the topmost modal while one is open
    let isShown = false;
    let home: Comment | undefined;
    const raise = (top: HTMLElement | undefined) => {
        // take the stack out of the top layer before moving it
        if (region === undefined || stack === undefined) {
            return;
        }
        if (isShown) {
            stack.hidePopover();
        }

        // place the region where a modal leaves it interactive and announced, else where it rendered
        if (top !== undefined && region.parentElement !== top) {
            top.append(region);
        } else if (top === undefined && home !== undefined && region.nextSibling !== home) {
            home.before(region);
        }

        // raise the stack above anything opened since
        isShown = untrack(manager.toasts).length > 0;
        if (isShown) {
            stack.showPopover();
        }
        // close a stack that leaves under the pointer, which sends no leave of its own
        else {
            standing.setHovered(false);
            standing.setFocused(false);
        }
    };
    let stop: (() => void) | undefined;
    onSettled(() => {
        // mark where the region rendered, and follow the document's modal stack
        if (region === undefined) {
            return;
        }
        const modals = LayerStack.of(region);
        home = region.ownerDocument.createComment("toaster");
        region.after(home);
        stop = modals.subscribe(raise);
        raise(modals.top());
    });
    onCleanup(() => {
        // stop following, and take the region out of any modal it moved into
        stop?.();
        home?.remove();
        region?.remove();
    });
    createEffect(
        () => manager.toasts()[0]?.id,
        () => raise(region === undefined ? undefined : LayerStack.of(region).top()),
    );

    // stand the newest toast at the edge of the page
    const edge = () => (toaster.position.startsWith("top") ? "top" : "bottom");

    return (
        <ToastContext value={manager}>
            {properties.children}
            <section
                ref={(element) => (region = element)}
                aria-label={locale.render(t`Notifications`)}
                aria-live="polite"
                aria-relevant="additions text"
                aria-atomic="false"
                tabindex={-1}
                data-slot="toaster"
                data-position={toaster.position}
                onFocusIn={(event) =>
                    standing.setFocused(
                        event.target instanceof Element && event.target.matches(":focus-visible"),
                    )
                }
                onFocusOut={() => standing.setFocused(false)}
            >
                <ol
                    ref={(element) => (stack = element)}
                    popover="manual"
                    data-slot="toaster-stack"
                    data-expanded={standing.isOpen() ? "" : undefined}
                    {...hover.attributes()}
                    {...style.attrs(
                        styles.toaster,
                        positions[toaster.position],
                        styles.height(`${String(standing.total())}px`),
                    )}
                >
                    <For each={shown()}>
                        {(entry) => (
                            <ToastItem
                                toast={entry}
                                manager={manager}
                                stack={standing}
                                focus={focus}
                                duration={toaster.duration}
                                edge={edge()}
                                swipe={swipeOf(toaster.position)}
                            />
                        )}
                    </For>
                </ol>
            </section>
        </ToastContext>
    );
}

/** The focus of a toaster: moved to it with F6, and back to where it was. */
class ToasterFocus {
    /** The element focused before the toasts. */
    #previous: HTMLElement | undefined;
    /** The region of the toasts. */
    readonly #region: () => HTMLElement | undefined;

    /** Listen for F6 once mounted. */
    constructor(region: () => HTMLElement | undefined) {
        // press F6 anywhere on the page
        this.#region = region;
        const listener = (event: KeyboardEvent) => this.#press(event);
        onSettled(() => {
            document.addEventListener("keydown", listener);

            return () => document.removeEventListener("keydown", listener);
        });
    }

    /** Move the focus to the toasts, or back to where it was from there. */
    #press(event: KeyboardEvent): void {
        // leave F6 to the page while no toast is shown
        const region = this.#region();
        const toast = region?.querySelector(STAYING);
        const isPressed = event.key === "F6" && !event.defaultPrevented;
        if (!isPressed || region === undefined || toast === null || toast === undefined) {
            return;
        }
        event.preventDefault();

        // go back from the toasts, else go to them
        const focused = document.activeElement;
        if (focused instanceof HTMLElement && region.contains(focused)) {
            this.restore();
        } else {
            this.#previous = focused instanceof HTMLElement ? focused : undefined;
            region.focus();
        }
    }

    /** Move the focus off a leaving toast: to the region while others stay, else back. */
    release(element: HTMLElement | undefined): void {
        // leave a focus elsewhere alone
        const region = this.#region();
        if (element === undefined || !element.contains(document.activeElement)) {
            return;
        }
        const others = region?.querySelectorAll(STAYING).length;
        if (others !== undefined && others > 1) {
            region?.focus();
        } else {
            this.restore();
        }
    }

    /** Move the focus back to where it was before the toasts. */
    restore(): void {
        // focus the element again, else leave the toasts for the page
        const previous = this.#previous;
        this.#previous = undefined;
        previous?.focus();
        const focused = document.activeElement;
        if (focused instanceof HTMLElement && this.#region()?.contains(focused) === true) {
            focused.blur();
        }
    }
}

/** Count down a time while nothing pauses it, calling back once it has run out. */
function followTime(duration: number, isPaused: Accessor<boolean>, onEnd: () => void): void {
    // keep the time left, and when the countdown last started
    const { clock } = useEnvironment();
    const timer = new Timeout();
    let remaining = duration;
    let started = 0;

    // stop and start with the pauses, never ending a toast that stays
    createEffect(isPaused, (pauses, paused) => {
        if (pauses && paused === false) {
            timer.clear();
            remaining -= clock.now() - started;
        } else if (!pauses && Number.isFinite(remaining)) {
            started = clock.now();
            timer.set(onEnd, remaining);
        }
    });
}

/** Read the time a toast shows: without end for an error or loading work, longer to act. */
function durationOf(entry: Toast, duration: number): number {
    if (entry.duration !== undefined) {
        return entry.duration;
    } else if (entry.type === "error" || entry.type === "loading") {
        return Number.POSITIVE_INFINITY;
    } else {
        return entry.action === undefined ? duration : Math.max(duration, ACTION_DURATION);
    }
}

/** Render one toast in its place, leaving after its duration unless the stack is open. */
function ToastItem(properties: {
    /** The toast. */
    readonly toast: Toast;
    /** The toasts. */
    readonly manager: ToastManager;
    /** The toasts as they stand. */
    readonly stack: ToastStack;
    /** The focus of the toaster. */
    readonly focus: ToasterFocus;
    /** The time it shows, in milliseconds. */
    readonly duration: number;
    /** The edge the newest toast stands at. */
    readonly edge: "top" | "bottom";
    /** The side a swipe dismisses it toward. */
    readonly swipe: Side;
}): JSX.Element {
    // dismiss it, moving the focus off it first
    const { manager, stack } = properties;
    const entry = properties.toast;
    let element: HTMLElement | undefined;
    const dismiss = () => {
        properties.focus.release(element);
        manager.dismiss(entry.id);
    };

    // count down its time while the stack is closed, the page is visible and it is in view
    const duration = durationOf(entry, properties.duration);
    const isVisible = usePageVisibility();
    const isLeaving = () => manager.leaving().has(entry.id);
    const isLimited = () => !isLeaving() && stack.isLimited(entry.id);
    followTime(duration, () => stack.isOpen() || !isVisible() || isLimited(), dismiss);

    // dismiss on a far swipe or a flick
    const swipe = new Swipe(
        {
            side: () => properties.swipe,
            onSwipeEnd: (event) => {
                if (event.isDismissal) {
                    dismiss();
                }
            },
        },
        useEnvironment().clock,
    );

    // measure its content, which its place in the stack depends on
    const [content, setContent] = createSignal<HTMLElement | undefined>(undefined, {
        ownedWrite: true,
    });
    createResizeObserver(content, (_rectangle, target) =>
        stack.measure(entry.id, target.offsetHeight),
    );

    // stand it behind the newer ones, and leave from where it stood last
    const place = createMemo<ToastPlace>((last) =>
        isLeaving() && last !== undefined ? last : stack.place(entry.id),
    );
    const transform = () => {
        // shift it away from the edge, sliding it off while it leaves
        const direction = properties.edge === "top" ? 1 : -1;
        const shift = `translateY(${String(direction * place().offset)}px)`;
        if (isLeaving()) {
            return `${shift} ${SLIDES[properties.swipe]}`;
        }

        return stack.isOpen() ? shift : `${shift} scale(${String(1 - place().index * SHRINK)})`;
    };
    const height = () => {
        const measured = place().height;

        return measured === undefined ? "auto" : `${String(measured)}px`;
    };

    return (
        <li
            ref={(created) => (element = created)}
            data-slot="toast"
            data-type={entry.type}
            data-leaving={isLeaving() ? "" : undefined}
            data-limited={isLimited() ? "" : undefined}
            inert={isLimited()}
            onKeyDown={(event) => {
                if (event.key === "Escape") {
                    event.preventDefault();
                    dismiss();
                }
            }}
            {...swipe.attributes()}
            {...style.attrs(
                text.sm,
                styles.toast,
                transition.notice,
                edges[properties.edge],
                styles.place(transform(), SLIDES[properties.swipe]),
                styles.layer(String(-place().index)),
                styles.height(height()),
                isLeaving() && transition.noticeLeaving,
                isLimited() && styles.limited,
                swipe.style(),
            )}
        >
            <div
                ref={setContent}
                {...style.attrs(
                    styles.content,
                    !stack.isOpen() && place().index > 0 && styles.behind,
                )}
            >
                <ToastIcon type={entry.type} />
                <div
                    role={entry.type === "error" ? "alert" : undefined}
                    {...style.attrs(styles.body)}
                >
                    <div data-slot="toast-title" {...style.attrs(styles.title)}>
                        {entry.title}
                    </div>
                    <Show when={entry.description}>
                        {(description) => (
                            <div data-slot="toast-description" {...style.attrs(styles.description)}>
                                {description()}
                            </div>
                        )}
                    </Show>
                </div>
                <ToastButtons
                    toast={entry}
                    isLasting={duration === Number.POSITIVE_INFINITY && entry.type !== "loading"}
                    onDismiss={dismiss}
                />
            </div>
        </li>
    );
}

/** Render the icon of a toast's kind, a spinner while loading and none by default. */
function ToastIcon(properties: {
    /** The kind. */
    readonly type: ToastType;
}): JSX.Element {
    const type = properties.type;
    if (type === "default") {
        return null;
    }

    return (
        <span aria-hidden="true" {...style.attrs(styles.icon, type !== "loading" && styles[type])}>
            {type === "loading" ? <Spinner /> : <Icon icon={ICONS[type]} />}
        </span>
    );
}

/** Render a toast's action, and a close button on one that stays. */
function ToastButtons(properties: {
    /** The toast. */
    readonly toast: Toast;
    /** Whether the toast stays until dismissed. */
    readonly isLasting: boolean;
    /** Dismiss the toast. */
    readonly onDismiss: () => void;
}): JSX.Element {
    const locale = useLocale();
    const entry = properties.toast;

    return (
        <div {...style.attrs(styles.buttons)}>
            <Show when={entry.action}>
                {(action) => (
                    <Button
                        variant="ghost"
                        size="sm"
                        data-slot="toast-action"
                        xstyle={styles.action}
                        onClick={(event) => {
                            action().onClick(event);
                            properties.onDismiss();
                        }}
                    >
                        {action().label}
                    </Button>
                )}
            </Show>
            <Show when={properties.isLasting}>
                <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label={locale.render(t`Close`)}
                    data-slot="toast-close"
                    onClick={() => properties.onDismiss()}
                >
                    <Icon name="x" />
                </Button>
            </Show>
        </div>
    );
}

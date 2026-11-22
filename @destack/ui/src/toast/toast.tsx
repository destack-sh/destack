import type { Clock } from "@destack/environment";
import { Icon } from "@destack/ui/icon";
import type { IconBodies } from "@destack/icon";
import checkCircle from "@destack/icon/phosphor/check-circle";
import info from "@destack/icon/phosphor/info";
import warning from "@destack/icon/phosphor/warning";
import xCircle from "@destack/icon/phosphor/x-circle";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import {
    color,
    radius,
    shadow,
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
    createEffect,
    createSignal,
    For,
    type JSX,
    merge,
    onCleanup,
    onSettled,
    type Setter,
    Show,
    untrack,
    useEnvironment,
    useLocale,
} from "@destack/view";
import { usePageVisibility } from "@destack/view/primitives/page-visibility";
import { Timeout } from "@destack/view/primitives/timer";
import { Button } from "../button/index.ts";
import { Spinner } from "../spinner/index.ts";
import { Hover, Swipe } from "../interaction/index.ts";
import { LayerStack } from "../layer/index.ts";
import type { Side } from "../position/index.ts";

/** The time a toast shows before it leaves on its own, in milliseconds. */
const DURATION = 4000;

/** The most toasts a toaster shows at once. */
const VISIBLE_TOASTS = 3;

/** The time a dismissed toast slides out before it unmounts, in milliseconds, as in Sonner. */
const LEAVING_TIME = 200;

/** The position and limits of a toaster that sets none. */
const DEFAULTS: Required<Pick<ToasterProperties, "position" | "duration" | "visibleToasts">> = {
    position: "bottom-right",
    duration: DURATION,
    visibleToasts: VISIBLE_TOASTS,
};

/** The icon of each kind of toast that shows one. */
const ICONS: Readonly<Record<"success" | "error" | "warning" | "info", IconBodies>> = {
    success: checkCircle,
    error: xCircle,
    warning,
    info,
};

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
    list: {
        display: "flex",
        flexDirection: "column",
        gap: space[2],
    },
    toast: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        padding: space[4],
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[4],
        backgroundColor: color.popover,
        color: color.foreground,
        boxShadow: shadow.overlay,
    },
    body: {
        display: "flex",
        flex: 1,
        flexDirection: "column",
        gap: space[1],
    },
    title: {
        fontWeight: weight.medium,
    },
    description: {
        color: color.mutedForeground,
    },
    success: { color: swatch.successMark },
    error: { color: swatch.destructiveMark },
    warning: { color: swatch.warningMark },
    info: { color: swatch.infoMark },
});

/** The surface of each kind of toast with rich colors. */
const riches = style.create({
    success: {
        borderColor: `color-mix(in oklab, ${swatch.successMark} 40%, transparent)`,
        backgroundColor: swatch.successTint,
    },
    error: {
        borderColor: `color-mix(in oklab, ${swatch.destructiveMark} 40%, transparent)`,
        backgroundColor: swatch.destructiveTint,
    },
    warning: {
        borderColor: `color-mix(in oklab, ${swatch.warningMark} 40%, transparent)`,
        backgroundColor: swatch.warningTint,
    },
    info: {
        borderColor: `color-mix(in oklab, ${swatch.infoMark} 40%, transparent)`,
        backgroundColor: swatch.infoTint,
    },
});

/** Return the rich surface of a kind of toast, none for the default and loading kinds. */
function richOf(type: ToastType): style.Styles {
    return type === "success" || type === "error" || type === "warning" || type === "info"
        ? riches[type]
        : null;
}

/** Read the side a swipe dismisses a toast toward, from where the toasts stack. */
function swipeOf(position: ToasterPosition): Side {
    if (position.endsWith("right")) {
        return "right";
    } else if (position.endsWith("left")) {
        return "left";
    }

    return position.startsWith("top") ? "top" : "bottom";
}

/** The slide of a toast entering from the side it is swiped away toward, after Sonner. */
const slides = style.create({
    top: {
        transform: { default: "none", "@starting-style": "translateY(-100%)" },
    },
    right: {
        transform: { default: "none", "@starting-style": "translateX(100%)" },
    },
    bottom: {
        transform: { default: "none", "@starting-style": "translateY(100%)" },
    },
    left: {
        transform: { default: "none", "@starting-style": "translateX(-100%)" },
    },
});

/** The slide of a dismissed toast back out toward the side it came from. */
const exits = style.create({
    top: { transform: "translateY(-100%)" },
    right: { transform: "translateX(100%)" },
    bottom: { transform: "translateY(100%)" },
    left: { transform: "translateX(-100%)" },
});

/** The corner of the viewport each position stacks toasts in. */
const positions = style.create({
    "top-left": { top: space[4], left: space[4] },
    "top-center": { top: space[4], left: "50%", transform: "translateX(-50%)" },
    "top-right": { top: space[4], right: space[4] },
    "bottom-left": { bottom: space[4], left: space[4] },
    "bottom-center": { bottom: space[4], left: "50%", transform: "translateX(-50%)" },
    "bottom-right": { bottom: space[4], right: space[4] },
});

/** The kind of a toast, which picks its icon and color. */
export type ToastType = "default" | "success" | "error" | "warning" | "info" | "loading";

/** The corner or edge of the viewport a toaster stacks its toasts in. */
export type ToasterPosition =
    | "top-left"
    | "top-center"
    | "top-right"
    | "bottom-left"
    | "bottom-center"
    | "bottom-right";

/** A button a toast shows beside its text. */
export interface ToastButton {
    /** The text of the button. */
    readonly label: string;
    /** Handle a click, after which the toast leaves. */
    readonly onClick: (event: MouseEvent) => void;
}

/** The options of a toast. */
export interface ToastOptions {
    /** The id of the toast, which a later toast with the same id replaces. */
    readonly id?: string;
    /** The text under the title. */
    readonly description?: string;
    /** The time the toast shows in milliseconds. */
    readonly duration?: number;
    /** The main button, such as undo. */
    readonly action?: ToastButton;
    /** The secondary button, such as dismiss. */
    readonly cancel?: ToastButton;
    /** Handle the toast leaving, by a person or on its own. */
    readonly onDismiss?: () => void;
}

/** A toast in the toaster. */
export interface Toast extends ToastOptions {
    /** The id of the toast. */
    readonly id: string;
    /** The kind of the toast. */
    readonly type: ToastType;
    /** The main text. */
    readonly title: string;
}

/** The texts a promise's toast shows while pending, once fulfilled and once rejected. */
export interface ToastPromiseMessages<Value> {
    /** The text while the promise is pending. */
    readonly loading: string;
    /** The text once the promise fulfils, or the text written from its value. */
    readonly success: string | ((value: Value) => string);
    /** The text once the promise rejects, or the text written from its reason. */
    readonly error: string | ((reason: unknown) => string);
}

/** The toasts every toaster on the page shows, newest first. */
export class ToastStore {
    /** The toasts, newest first, the dismissed ones until they have slid out. */
    readonly toasts: Accessor<readonly Toast[]>;
    /** The ids of the dismissed toasts still sliding out. */
    readonly leaving: Accessor<ReadonlySet<string>>;
    /** Replace the toasts. */
    readonly #setToasts: Setter<readonly Toast[]>;
    /** Replace the ids of the toasts sliding out. */
    readonly #setLeaving: Setter<ReadonlySet<string>>;
    /** The number of toasts shown so far, which numbers the next toast without an id. */
    #count: number;

    /** Create an empty store. */
    constructor() {
        // start without toasts
        const [toasts, setToasts] = createSignal<readonly Toast[]>([]);
        const [leaving, setLeaving] = createSignal<ReadonlySet<string>>(new Set());
        this.toasts = toasts;
        this.leaving = leaving;
        this.#setToasts = setToasts;
        this.#setLeaving = setLeaving;
        this.#count = 0;
    }

    /** Show a toast, replacing the one with the same id in place, and return its id. */
    show(title: string, type: ToastType, options: ToastOptions = {}): string {
        // replace the toast with the same id, else put it first
        this.#count += 1;
        const id = options.id ?? `toast-${this.#count}`;
        const shown: Toast = { ...options, id, type, title };
        this.#setLeaving((ids) => new Set([...ids].filter((each) => each !== id)));
        this.#setToasts((toasts) =>
            toasts.some((entry) => entry.id === id)
                ? toasts.map((entry) => (entry.id === id ? shown : entry))
                : [shown, ...toasts],
        );

        return id;
    }

    /** Dismiss a toast, or every toast without an id, sliding each out and telling it. */
    dismiss(id?: string): void {
        const leaving = this.toasts().filter((entry) => id === undefined || entry.id === id);
        this.#setLeaving((ids) => new Set([...ids, ...leaving.map((entry) => entry.id)]));
        for (const entry of leaving) {
            entry.onDismiss?.();
        }
    }

    /** Remove a dismissed toast once it has slid out, unless it was shown again meanwhile. */
    remove(id: string): void {
        if (!this.leaving().has(id)) {
            return;
        }
        this.#setLeaving((ids) => new Set([...ids].filter((each) => each !== id)));
        this.#setToasts((toasts) => toasts.filter((entry) => entry.id !== id));
    }
}

/** The store of the page's toasts. */
const store = new ToastStore();

/** Show a toast with a title, returning its id. */
function show(title: string, options?: ToastOptions): string {
    return store.show(title, "default", options);
}

/** Show a loading toast while a promise is pending, and its outcome once it settles. */
function follow<Value>(
    promise: Promise<Value>,
    messages: ToastPromiseMessages<Value>,
    options?: ToastOptions,
): Promise<Value> {
    const id = store.show(messages.loading, "loading", options);
    void promise.then(
        (value) =>
            store.show(
                typeof messages.success === "string" ? messages.success : messages.success(value),
                "success",
                { ...options, id },
            ),
        (reason: unknown) =>
            store.show(
                typeof messages.error === "string" ? messages.error : messages.error(reason),
                "error",
                { ...options, id },
            ),
    );

    return promise;
}

/** Show toasts from anywhere on the page. */
export const toast = Object.assign(show, {
    /** Show a toast of a success. */
    success: (title: string, options?: ToastOptions) => store.show(title, "success", options),
    /** Show a toast of an error. */
    error: (title: string, options?: ToastOptions) => store.show(title, "error", options),
    /** Show a toast of a warning. */
    warning: (title: string, options?: ToastOptions) => store.show(title, "warning", options),
    /** Show a toast of information. */
    info: (title: string, options?: ToastOptions) => store.show(title, "info", options),
    /** Show a toast of work in progress that stays until replaced or dismissed. */
    loading: (title: string, options?: ToastOptions) => store.show(title, "loading", options),
    /** Follow a promise with a loading toast that turns into its outcome. */
    promise: follow,
    /** Dismiss a toast by id, or every toast. */
    dismiss: (id?: string) => store.dismiss(id),
});

/** The properties of a toaster. */
export interface ToasterProperties {
    /** The corner or edge the toasts stack in, bottom right by default. */
    readonly position?: ToasterPosition;
    /** The time each toast shows in milliseconds, 4000 by default. */
    readonly duration?: number;
    /** The most toasts shown at once, 3 by default. */
    readonly visibleToasts?: number;
    /** Whether each kind of toast takes its color across its surface, not only its icon. */
    readonly richColors?: boolean;
}

/** Render the live region the page's toasts appear and are announced in, focused with Alt+T. */
export function Toaster(properties: ToasterProperties): JSX.Element {
    // read the placement and the locale
    const toaster = merge(DEFAULTS, properties);
    const locale = useLocale();
    let region: HTMLElement | undefined;
    let stack: HTMLElement | undefined;

    // move the focus to the toasts on Alt+T
    focusOnShortcut(() => region);

    // drop the toasts dismissed before this toaster showed them, which have no exit to play
    onSettled(() => {
        for (const id of store.leaving()) {
            store.remove(id);
        }
    });

    // unmount each dismissed toast once it has slid out
    const { clock } = useEnvironment();
    createEffect(store.leaving, (ids, before) => {
        for (const id of ids) {
            if (before?.has(id) !== true) {
                clock.setTimeout(() => store.remove(id), LEAVING_TIME);
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
        isShown = untrack(store.toasts).length > 0;
        if (isShown) {
            stack.showPopover();
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
        () => store.toasts()[0]?.id,
        () => raise(region === undefined ? undefined : LayerStack.of(region).top()),
    );

    return (
        <section
            ref={(element) => (region = element)}
            aria-label={locale.render(t`Notifications`)}
            aria-live="polite"
            aria-relevant="additions text"
            aria-atomic="false"
            tabindex={-1}
            data-slot="toaster"
            data-position={toaster.position}
        >
            <ol
                ref={(element) => (stack = element)}
                popover="manual"
                data-slot="toaster-stack"
                {...style.attrs(styles.toaster, styles.list, positions[toaster.position])}
            >
                <For each={store.toasts().slice(0, toaster.visibleToasts)}>
                    {(entry) => (
                        <ToastItem
                            toast={entry}
                            duration={entry.duration ?? toaster.duration}
                            swipe={swipeOf(toaster.position)}
                            isRich={toaster.richColors === true}
                        />
                    )}
                </For>
            </ol>
        </section>
    );
}

/** Move the focus to an element on Alt+T once mounted. */
function focusOnShortcut(target: () => HTMLElement | undefined): void {
    // focus the target on Alt+T
    const focus = (event: KeyboardEvent) => {
        if (event.altKey && event.code === "KeyT") {
            event.preventDefault();
            target()?.focus();
        }
    };

    // listen for the shortcut once mounted
    onSettled(() => {
        document.addEventListener("keydown", focus);

        return () => document.removeEventListener("keydown", focus);
    });
}

/** The time a toast has left to show. */
class ToastTimer {
    /** The time left, in milliseconds, infinite for a toast that stays. */
    #remaining: number;
    /** The time the countdown last started, on the page's clock. */
    #started = 0;
    /** The page's clock. */
    readonly #clock: Clock = useEnvironment().clock;
    /** The pending end of the countdown. */
    readonly #timer = new Timeout();
    /** The pointer, the focus and a hidden page, each pausing the countdown. */
    #pauses = 0;
    /** End the toast once its time runs out. */
    readonly #onEnd: () => void;

    /** Start counting down a time, cancelling the countdown when its owner disposes. */
    constructor(duration: number, onEnd: () => void) {
        // start the countdown, stopping it as the toast leaves
        this.#remaining = duration;
        this.#onEnd = onEnd;
        this.#run();
    }

    /** Pause the countdown while the pointer or the focus rests on the toast. */
    pause(): void {
        this.#pauses += 1;
        if (this.#pauses === 1) {
            this.#timer.clear();
            this.#remaining -= this.#clock.now() - this.#started;
        }
    }

    /** Release a pause. */
    resume(): void {
        this.#pauses -= 1;
        if (this.#pauses === 0) {
            this.#run();
        }
    }

    /** Count down the time left, unless the toast stays. */
    #run(): void {
        if (Number.isFinite(this.#remaining)) {
            this.#started = this.#clock.now();
            this.#timer.set(this.#onEnd, this.#remaining);
        }
    }
}

/** Render one toast, leaving after its duration unless the pointer or focus rests on it. */
function ToastItem(properties: {
    /** The toast. */
    readonly toast: Toast;
    /** The time it shows, in milliseconds. */
    readonly duration: number;
    /** The side a swipe dismisses it toward. */
    readonly swipe: Side;
    /** Whether its kind colors its surface. */
    readonly isRich: boolean;
}): JSX.Element {
    // count down the toast's time, a loading toast staying
    const locale = useLocale();
    const entry = properties.toast;
    const timer = new ToastTimer(
        entry.type === "loading" ? Number.POSITIVE_INFINITY : properties.duration,
        () => store.dismiss(entry.id),
    );

    // pause the countdown while the page is hidden, as Sonner does
    const isVisible = usePageVisibility();
    createEffect(isVisible, (isShown, wasShown) => {
        if (!isShown) {
            timer.pause();
        } else if (wasShown === false) {
            timer.resume();
        }
    });
    const swipe = new Swipe(
        {
            side: () => properties.swipe,
            onSwipeEnd: (event) => {
                // dismiss on a far swipe or a flick
                if (event.isDismissal) {
                    store.dismiss(entry.id);
                }
            },
        },
        useEnvironment().clock,
    );
    const hover = new Hover({
        onHoverStart: () => timer.pause(),
        onHoverEnd: () => timer.resume(),
    });

    return (
        <li
            data-slot="toast"
            data-type={entry.type}
            data-leaving={store.leaving().has(entry.id) ? "" : undefined}
            {...hover.attributes()}
            onFocusIn={() => timer.pause()}
            onFocusOut={() => timer.resume()}
            {...swipe.attributes()}
            {...style.attrs(
                text.xs,
                styles.toast,
                transition.notice,
                slides[properties.swipe],
                properties.isRich && richOf(entry.type),
                store.leaving().has(entry.id) && [
                    transition.noticeLeaving,
                    exits[properties.swipe],
                ],
                swipe.style(),
            )}
        >
            <ToastIcon type={entry.type} />
            <div {...style.attrs(styles.body)}>
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
            <ToastButtons toast={entry} />
            <Button
                variant="ghost"
                size="icon-sm"
                aria-label={locale.render(t`Close`)}
                data-slot="toast-close"
                onClick={() => store.dismiss(entry.id)}
            >
                <Icon name="x" />
            </Button>
        </li>
    );
}

/** Render the icon of a toast's kind, a spinner while loading and none by default. */
function ToastIcon(properties: { readonly type: ToastType }): JSX.Element {
    const type = properties.type;
    if (type === "loading") {
        return <Spinner />;
    } else if (type === "default") {
        return null;
    } else {
        return (
            <span aria-hidden="true" {...style.attrs(styles[type])}>
                <Icon icon={ICONS[type]} />
            </span>
        );
    }
}

/** Render a toast's cancel and action buttons, each dismissing the toast after its handler. */
function ToastButtons(properties: { readonly toast: Toast }): JSX.Element {
    const entry = properties.toast;

    return (
        <>
            <Show when={entry.cancel}>
                {(cancel) => (
                    <Button
                        variant="outline"
                        size="sm"
                        data-slot="toast-cancel"
                        onClick={(event) => {
                            cancel().onClick(event);
                            store.dismiss(entry.id);
                        }}
                    >
                        {cancel().label}
                    </Button>
                )}
            </Show>
            <Show when={entry.action}>
                {(action) => (
                    <Button
                        size="sm"
                        data-slot="toast-action"
                        onClick={(event) => {
                            action().onClick(event);
                            store.dismiss(entry.id);
                        }}
                    >
                        {action().label}
                    </Button>
                )}
            </Show>
        </>
    );
}

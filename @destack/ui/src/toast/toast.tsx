import { Icon, type IconBodies } from "@destack/icon";
import checkCircle from "@destack/icon/phosphor/check-circle";
import info from "@destack/icon/phosphor/info";
import warning from "@destack/icon/phosphor/warning";
import xCircle from "@destack/icon/phosphor/x-circle";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, radius, shadow, space, stroke, weight, width } from "@destack/theme/tokens.stylex";
import { useLocale } from "@destack/locale/solid";
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
} from "@destack/view";
import { Button } from "../button/index.ts";
import { Spinner } from "../spinner/index.ts";
import { createSwipe, type SwipeDirection, swipeStyle } from "../swipe/index.ts";

/** The time a toast shows before it leaves on its own, in milliseconds. */
const DURATION = 4000;

/** The most toasts a toaster shows at once. */
const VISIBLE_TOASTS = 3;

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
        margin: 0,
        padding: 0,
        overflow: "visible",
        borderWidth: 0,
        backgroundColor: "transparent",
        color: "inherit",
        outlineStyle: "none",
    },
    list: {
        display: "flex",
        flexDirection: "column",
        gap: space[2],
        margin: 0,
        padding: 0,
        listStyle: "none",
    },
    toast: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        padding: space[4],
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[4],
        backgroundColor: color.popover,
        color: color.popoverForeground,
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
    success: { color: color.success },
    error: { color: color.destructive },
    warning: { color: color.warning },
    info: { color: color.info },
});

/** The surface of each kind of toast with rich colors. */
const riches = style.create({
    success: {
        borderColor: `color-mix(in oklab, ${color.success} 40%, transparent)`,
        backgroundColor: `color-mix(in oklab, ${color.success} 12%, ${color.popover})`,
    },
    error: {
        borderColor: `color-mix(in oklab, ${color.destructive} 40%, transparent)`,
        backgroundColor: `color-mix(in oklab, ${color.destructive} 12%, ${color.popover})`,
    },
    warning: {
        borderColor: `color-mix(in oklab, ${color.warning} 40%, transparent)`,
        backgroundColor: `color-mix(in oklab, ${color.warning} 12%, ${color.popover})`,
    },
    info: {
        borderColor: `color-mix(in oklab, ${color.info} 40%, transparent)`,
        backgroundColor: `color-mix(in oklab, ${color.info} 12%, ${color.popover})`,
    },
});

/** Return the rich surface of a kind of toast, none for the default and loading kinds. */
function richOf(type: ToastType): style.Styles {
    return type === "success" || type === "error" || type === "warning" || type === "info"
        ? riches[type]
        : null;
}

/** Read the direction a swipe dismisses a toast toward from where the toasts stack: off the nearer side. */
function swipeOf(position: ToasterPosition): SwipeDirection {
    if (position.endsWith("right")) {
        return "right";
    } else if (position.endsWith("left")) {
        return "left";
    }

    return position.startsWith("top") ? "up" : "down";
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
    /** The time the toast shows in milliseconds, the toaster's by default and forever for Infinity. */
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
    /** The toasts, newest first. */
    readonly toasts: Accessor<readonly Toast[]>;
    /** Replace the toasts. */
    readonly #setToasts: Setter<readonly Toast[]>;
    /** The number of toasts shown so far, which names the next toast without an id. */
    #count: number;

    /** Create an empty store. */
    constructor() {
        // start without toasts
        const [toasts, setToasts] = createSignal<readonly Toast[]>([]);
        this.toasts = toasts;
        this.#setToasts = setToasts;
        this.#count = 0;
    }

    /** Show a toast, replacing the one with the same id in place, and return its id. */
    show(title: string, type: ToastType, options: ToastOptions = {}): string {
        // name the toast and put it in place of its namesake or first
        this.#count += 1;
        const id = options.id ?? `toast-${this.#count}`;
        const shown: Toast = { ...options, id, type, title };
        this.#setToasts((toasts) =>
            toasts.some((entry) => entry.id === id)
                ? toasts.map((entry) => (entry.id === id ? shown : entry))
                : [shown, ...toasts],
        );

        return id;
    }

    /** Remove a toast, or every toast without an id, telling each its dismissal. */
    dismiss(id?: string): void {
        const leaving = this.toasts().filter((entry) => id === undefined || entry.id === id);
        this.#setToasts((toasts) => toasts.filter((entry) => !leaving.includes(entry)));
        for (const entry of leaving) {
            entry.onDismiss?.();
        }
    }
}

/** The store of the page's toasts. */
const store = new ToastStore();

/** Show a toast with a title, returning its id. */
function show(title: string, options?: ToastOptions): string {
    return store.show(title, "default", options);
}

/** Show a loading toast while a promise is pending and its outcome once it settles, returning the promise. */
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

    // keep the stack in the top layer while toasts exist, raising it above anything opened since
    let isShown = false;
    createEffect(
        () => store.toasts()[0]?.id,
        (newest) => {
            if (stack === undefined) {
                return;
            }
            if (isShown) {
                stack.hidePopover();
            }
            isShown = newest !== undefined;
            if (isShown) {
                stack.showPopover();
            }
        },
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

/** Move the focus to an element on Alt+T once mounted, by the key's position whatever letter the layout types. */
function focusOnShortcut(target: () => HTMLElement | undefined): void {
    onSettled(() => {
        const focus = (event: KeyboardEvent) => {
            if (event.altKey && event.code === "KeyT") {
                event.preventDefault();
                target()?.focus();
            }
        };
        document.addEventListener("keydown", focus);

        return () => document.removeEventListener("keydown", focus);
    });
}

/** Render one toast, leaving after its duration unless the pointer or focus rests on it. */
function ToastItem(properties: {
    /** The toast. */
    readonly toast: Toast;
    /** The time it shows, in milliseconds. */
    readonly duration: number;
    /** The direction a swipe dismisses it toward. */
    readonly swipe: SwipeDirection;
    /** Whether its kind colors its surface. */
    readonly isRich: boolean;
}): JSX.Element {
    // count down the toast's time, pausing while the pointer or focus is on it
    const locale = useLocale();
    const entry = properties.toast;
    let remaining = entry.type === "loading" ? Number.POSITIVE_INFINITY : properties.duration;
    let started = 0;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const resume = () => {
        if (Number.isFinite(remaining)) {
            started = performance.now();
            timer = setTimeout(() => store.dismiss(entry.id), remaining);
        }
    };
    const pause = () => {
        clearTimeout(timer);
        remaining -= performance.now() - started;
    };
    resume();
    onCleanup(() => clearTimeout(timer));
    const swipe = createSwipe(
        () => properties.swipe,
        () => store.dismiss(entry.id),
    );

    return (
        <li
            data-slot="toast"
            data-type={entry.type}
            data-swiping={swipe.offset() === undefined ? undefined : ""}
            onPointerEnter={pause}
            onPointerLeave={resume}
            onFocusIn={pause}
            onFocusOut={resume}
            onPointerDown={swipe.onPointerDown}
            onPointerMove={swipe.onPointerMove}
            onPointerUp={swipe.onPointerUp}
            onPointerCancel={swipe.onPointerCancel}
            {...style.attrs(
                text.footnote,
                styles.toast,
                properties.isRich && richOf(entry.type),
                swipeStyle(swipe.translate()),
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
                size="icon-xs"
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
                        size="xs"
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
                        size="xs"
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

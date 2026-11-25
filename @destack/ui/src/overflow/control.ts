import type { Environment } from "@destack/environment";
import {
    type Accessor,
    createContext,
    createEffect,
    createSignal,
    flush,
    untrack,
    useContext,
} from "@destack/view";
import { CollectionBuilder } from "../collection/index.ts";
import type { Axis } from "../position/index.ts";

/** What a person presses in an item. */
const TARGET = "button, a[href], [role=button], [role=tab], [role=menuitem]";

/** What stays shown: what is selected, the current page, and what has something open. */
const ACTIVE =
    "[aria-selected=true], [aria-current]:not([aria-current=false]), [aria-expanded=true]";

/** The attributes that say so. */
const ACTIVE_ATTRIBUTES = ["aria-selected", "aria-current", "aria-expanded"];

/** The pixels by which browsers round a length. */
const ROUNDING = 1;

/** What an entry is: something that may hide, the menu of what hides, or a mark between two. */
export type OverflowKind = "item" | "menu" | "separator";

/** What sets an entry of an overflow apart. */
export interface OverflowEntryOptions {
    /** What it is. */
    readonly kind: OverflowKind;
    /** Read its element, none before it mounts. */
    readonly element: () => HTMLElement | undefined;
    /** Read the order in which it hides, the lowest first, none for zero. */
    readonly priority?: () => number | undefined;
    /** Read what the menu calls it, none for the words of its element. */
    readonly label?: () => string | undefined;
}

/** Something that shows or hides in an overflow. */
export class OverflowEntry {
    /** What it is, a menu where it has the menu in it. */
    kind: OverflowKind;
    /** What sets it apart. */
    readonly #options: OverflowEntryOptions;
    /** The overflow it stands in. */
    readonly #control: OverflowControl;

    /** Create an entry of an overflow. */
    constructor(control: OverflowControl, options: OverflowEntryOptions) {
        this.kind = options.kind;
        this.#options = options;
        this.#control = control;
    }

    /** Read its element, none before it mounts. */
    element(): HTMLElement | undefined {
        return this.#options.element();
    }

    /** Read the order in which it hides, the lowest first. */
    priority(): number {
        return this.#options.priority?.() ?? 0;
    }

    /** Report whether it does not fit. */
    isHidden(): boolean {
        return this.#control.hidden().includes(this);
    }

    /** Read what a person presses in it, none where it only shows something. */
    target(): HTMLElement | undefined {
        const element = this.element();

        return element?.matches(TARGET) === true
            ? element
            : (element?.querySelector<HTMLElement>(TARGET) ?? undefined);
    }

    /** Read what it is called, by its owner or by its element. */
    label(): string {
        const named = this.target() ?? this.element();

        return (
            this.#options.label?.() ??
            named?.getAttribute("aria-label") ??
            named?.textContent?.trim() ??
            ""
        );
    }

    /** Report whether it takes no press. */
    isDisabled(): boolean {
        return this.target()?.matches(":disabled, [aria-disabled=true]") !== false;
    }

    /** Report whether it is pressed or checked, undefined for what is neither. */
    isChecked(): boolean | undefined {
        const target = this.target();
        const state = target?.getAttribute("aria-pressed") ?? target?.getAttribute("aria-checked");

        return state === null || state === undefined ? undefined : state === "true";
    }

    /** Report whether it stays shown: it is selected, current or open, or it has the focus. */
    isActive(): boolean {
        const element = this.element();

        return (
            element !== undefined &&
            (element.matches(ACTIVE) ||
                element.querySelector(ACTIVE) !== null ||
                element.contains(element.ownerDocument.activeElement))
        );
    }

    /** Press it, showing it first where it opens something or takes the focus. */
    press(): void {
        const target = this.target();
        if (target?.matches("[aria-haspopup], [role=tab]") === true) {
            this.#control.show(this);
            target.focus();
        }
        target?.click();
    }
}

/** The overflow around an element, none outside one. */
export const OverflowContext = createContext<OverflowControl | null>(null);

/** The entry around an element, whose own parts then join no more. */
export const OverflowEntryContext = createContext<OverflowEntry | null>(null);

/** Read the nearest overflow, refusing elements outside one. */
export function useOverflow(): OverflowControl {
    const control = useContext(OverflowContext);
    if (control === null) {
        throw new TypeError("overflow elements need an overflow around them");
    }

    return control;
}

/** Join the overflow around an element until it unmounts, none outside one or in an entry. */
export function useOverflowEntry(options: OverflowEntryOptions): OverflowEntry | undefined {
    const control = useContext(OverflowContext);
    const around = useContext(OverflowEntryContext);

    return control === null || around !== null ? undefined : control.add(options);
}

/** What fits along an axis, and what hides. */
export class OverflowControl {
    /** The entries that do not fit, in the order of their elements. */
    readonly hidden: Accessor<readonly OverflowEntry[]>;
    /** The entries in the order of their elements. */
    readonly #entries: CollectionBuilder<OverflowEntry>;
    /** Read the axis they line up along. */
    readonly #axis: () => Axis;
    /** The display whose next frame a change of an entry waits for. */
    readonly #environment: Environment<"display">;
    /** Replace the entries that do not fit. */
    readonly #setHidden: (hidden: readonly OverflowEntry[]) => void;
    /** The element they line up in. */
    #element: HTMLElement | undefined;
    /** Whether a frame is on its way. */
    #isScheduled = false;

    /** Start with everything shown. */
    constructor(axis: () => Axis, environment: Environment<"display">) {
        // show everything until measured
        const [hidden, setHidden] = createSignal<readonly OverflowEntry[]>([], {
            ownedWrite: true,
            equals: (one, other) =>
                one.length === other.length && one.every((each, index) => each === other[index]),
        });
        this.hidden = hidden;
        this.#setHidden = setHidden;
        this.#axis = axis;
        this.#environment = environment;
        this.#entries = new CollectionBuilder((entry) => entry.element());
        this.#follow();
    }

    /** Add an entry until the component adding it unmounts. */
    add(options: OverflowEntryOptions): OverflowEntry {
        const entry = new OverflowEntry(this, options);
        this.#entries.add(entry);

        return entry;
    }

    /** Take the element they line up in. */
    attach(element: HTMLElement): void {
        this.#element = element;
    }

    /** Show an entry until the next measure, at once. */
    show(entry: OverflowEntry): void {
        // show its element, and tell what reads it
        const element = entry.element();
        if (element !== undefined) {
            element.hidden = false;
        }
        this.#setHidden(untrack(this.hidden).filter((each) => each !== entry));
        flush();
    }

    /** Hide what does not fit, the rest standing as it is. */
    measure(): void {
        // wait for the element
        const container = this.#element;
        if (container?.isConnected !== true) {
            return;
        }

        // find the fewest to hide by trying, as the browser alone knows what fits
        const entries = untrack(() => this.#entries.items());
        const order = untrack(() => this.#order(entries));
        let hidden: ReadonlySet<OverflowEntry> = new Set();
        if (!this.#isWholeFitting(container, entries)) {
            let [least, most] = [1, order.length];
            while (least < most) {
                const middle = Math.floor((least + most) / 2);
                this.#draw(entries, new Set(order.slice(0, middle)));
                if (this.#isFitting(container)) {
                    most = middle;
                } else {
                    least = middle + 1;
                }
            }
            hidden = new Set(order.slice(0, least));
        }

        // draw those that fit, and tell what reads the others
        this.#draw(entries, hidden);
        this.#setHidden(entries.filter((entry) => hidden.has(entry)));
    }

    /** Measure again as the room, the entries, their size or what is active changes. */
    #follow(): void {
        createEffect(
            () => this.#entries.items().flatMap((entry) => entry.element() ?? []),
            (elements) => {
                // hide what does not fit before the browser draws it
                const container = this.#element;
                if (container === undefined) {
                    return undefined;
                }
                this.measure();

                // follow the room with the next frame, as a change inside a report repeats it,
                //  and what is around it, as what fits its content grows back with it
                const resizes = new ResizeObserver(() => this.#schedule());
                for (
                    let each: Element | null = container;
                    each !== null;
                    each = each.parentElement
                ) {
                    resizes.observe(each);
                }
                elements.forEach((element) => resizes.observe(element));

                // follow what is active
                const changes = new MutationObserver(() => this.measure());
                changes.observe(container, {
                    subtree: true,
                    attributeFilter: ACTIVE_ATTRIBUTES,
                });
                const listening = new AbortController();
                container.addEventListener("focusout", () => this.#schedule(), {
                    signal: listening.signal,
                });

                return () => {
                    resizes.disconnect();
                    changes.disconnect();
                    listening.abort();
                };
            },
        );
    }

    /** Measure with the next frame, once. */
    #schedule(): void {
        if (!this.#isScheduled) {
            this.#isScheduled = true;
            void this.#environment.display.frame().then(() => {
                this.#isScheduled = false;
                this.measure();
            });
        }
    }

    /** List what may hide: by priority, then onward from the menu, then back from it. */
    #order(entries: readonly OverflowEntry[]): readonly OverflowEntry[] {
        // start at the menu, else at the end
        const menu = entries.findIndex((entry) => entry.kind === "menu");
        const place = menu === -1 ? entries.length : menu;
        const candidates = [...entries.slice(place), ...entries.slice(0, place).toReversed()];

        return candidates
            .filter(
                (entry) =>
                    entry.kind === "item" && entry.priority() < Infinity && !entry.isActive(),
            )
            .toSorted((one, other) => one.priority() - other.priority());
    }

    /** Show all but the hidden entries, a menu for those, and separators between. */
    #draw(
        entries: readonly OverflowEntry[],
        hidden: ReadonlySet<OverflowEntry>,
        standing?: HTMLElement,
    ): void {
        // start without anything before a separator
        let pending: HTMLElement | undefined;
        let isFirst = true;
        for (const entry of entries) {
            const element = entry.element();

            // leave out what has not mounted
            if (element === undefined) {
                continue;
            }
            // hide a separator until something shows after it
            else if (entry.kind === "separator") {
                element.hidden = true;
                pending = isFirst ? undefined : (pending ?? element);
            }
            // show what fits, and the first separator before it
            else {
                const isShown = entry.kind === "menu" ? hidden.size > 0 : !hidden.has(entry);
                element.hidden = !isShown && element !== standing;
                if (isShown && pending !== undefined) {
                    pending.hidden = false;
                }
                pending = isShown ? undefined : pending;
                isFirst = isFirst && !isShown;
            }
        }
    }

    /** Report whether everything fits without the menu. */
    #isWholeFitting(container: HTMLElement, entries: readonly OverflowEntry[]): boolean {
        // take a menu with the focus out of the way, as hiding it drops the focus
        const menu = entries.find((entry) => entry.kind === "menu")?.element();
        const isFocused = menu?.contains(menu.ownerDocument.activeElement) === true;
        if (menu === undefined || !isFocused) {
            this.#draw(entries, new Set());

            return this.#isFitting(container);
        }
        const { position } = menu.style;
        menu.style.position = "fixed";
        this.#draw(entries, new Set(), menu);
        const isFitting = this.#isFitting(container);
        menu.style.position = position;

        return isFitting;
    }

    /** Report whether the entries and what stands beside them fit along the axis. */
    #isFitting(element: HTMLElement): boolean {
        // compare by layout alone, which moves under way leave as it will be
        const isInline = untrack(this.#axis) === "horizontal";
        const room = isInline ? element.clientWidth : element.clientHeight;
        const own = untrack(() => this.#entries.items()).flatMap((entry) => entry.element() ?? []);
        // leave out what stands over the row, such as an open menu
        const beside = [...element.children].flatMap((each) =>
            each instanceof HTMLElement &&
            !each.matches("[popover]") &&
            getComputedStyle(each).position !== "fixed"
                ? [each]
                : [],
        );

        return [...own, ...beside].every((each) => {
            const start = startWithin(each, element, isInline);
            const length = isInline ? each.offsetWidth : each.offsetHeight;

            return length === 0 || (start >= -ROUNDING && start + length <= room + ROUNDING);
        });
    }
}

/** Read how far an element stands from the start of another along an axis, by layout alone. */
function startWithin(element: HTMLElement, around: HTMLElement, isInline: boolean): number {
    // add up the offsets to what both are placed in
    const offset = (each: HTMLElement): number => (isInline ? each.offsetLeft : each.offsetTop);
    let start = 0;
    let each: Element | null = element;
    while (each instanceof HTMLElement && each !== around && around.contains(each)) {
        start += offset(each);
        each = each.offsetParent;
    }

    return each === around ? start : start - offset(around);
}

import {
    type Accessor,
    createContext,
    createControllableSignal,
    createEffect,
    createSignal,
    createUniqueId,
    flush,
    handle,
    untrack,
    useContext,
} from "@destack/view";
import type { Direction } from "@destack/locale";
import type { Action } from "@destack/object";
import { CollectionBuilder } from "../collection/index.ts";
import { ListState } from "../focus/index.ts";
import {
    closeAllTabs,
    closeOtherTabs,
    closeTab,
    closeTabsAfter,
    goToTab,
    moveTabLeft,
    moveTabRight,
    nextTab,
    previousTab,
    reopenTab,
} from "./actions.ts";
import type { TabsProperties } from "./tabs.tsx";

/** The tabs of the nearest tabs root, null outside one. */
export const TabsContext = createContext<TabsControl | null>(null);

/** The item around a tab, null for a tab by itself. */
export const TabsItemContext = createContext<TabsItemControl | null>(null);

/** Whether moving the focus to a tab selects it, or Enter and Space do. */
export type TabsActivationMode = "automatic" | "manual";

/** Where a tab is in its list, in pixels. */
export interface TabsPlace {
    /** The distance from the list's left. */
    readonly left: number;
    /** The distance from the list's top. */
    readonly top: number;
    /** The tab's width. */
    readonly width: number;
    /** The tab's height. */
    readonly height: number;
}

/** A tab with what is beside it. */
export interface TabsItemControl {
    /** The value the tab and its panel stand for. */
    readonly value: string;
    /** Read its element, none before it mounts. */
    readonly element: () => HTMLElement | undefined;
    /** Report whether it has a button that closes it. */
    readonly isClosable: () => boolean;
    /** Add a button that closes it, until the button unmounts. */
    readonly addClose: () => void;
    /** Whether it hides in an overflow. */
    readonly isHidden: () => boolean;
}

/** The selected tab, ids and keyboard model of a set of tabs. */
export class TabsControl {
    /** The properties of the tabs root, read for its controlled state and keyboard model. */
    readonly properties: Required<Pick<TabsProperties, "orientation" | "activationMode">> &
        TabsProperties;
    /** The selected value, controlled or the root's own. */
    readonly value: Accessor<string | undefined>;
    /** Where the indicator is, undefined without one. */
    readonly place: Accessor<TabsPlace | undefined>;
    /** Replace where the indicator is. */
    readonly setPlace: (place: TabsPlace | undefined) => void;
    /** The items around tabs. */
    readonly items: CollectionBuilder<TabsItemControl>;
    /** The tabs in document order and the one with the tab stop. */
    readonly list: ListState;
    /** The prefix of every id of the tabs and panels. */
    readonly #prefix: string;
    /** Replace the selected value and tell the change handler. */
    readonly #setValue: (value: string | undefined) => void;

    /** Create the state of a set of tabs from its root's properties. */
    constructor(
        properties: Required<Pick<TabsProperties, "orientation" | "activationMode">> &
            TabsProperties,
    ) {
        // start from the default value with no tabs yet
        const [value, setValue] = createControllableSignal({
            isControlled: () => properties.value !== undefined,
            value: () => properties.value,
            defaultValue: properties.defaultValue,
            onChange: (next) => {
                // tell the change handler of each selected tab
                if (next !== undefined) {
                    properties.onValueChange?.(next);
                }
            },
        });
        this.properties = properties;
        this.value = value;
        this.list = new ListState({
            get orientation() {
                return properties.orientation;
            },
            isLooping: true,
            isTypeahead: false,
            initial: value,
        });
        this.#prefix = untrack(() => properties.idPrefix) ?? createUniqueId();
        this.#setValue = setValue;
        this.items = new CollectionBuilder(undefined, (item) => item.value);
        const [place, setPlace] = createSignal<TabsPlace | undefined>(undefined, {
            ownedWrite: true,
        });
        this.place = place;
        this.setPlace = setPlace;

        // select the tab beside a selected one that is removed, as its focus does
        createEffect(
            () => this.list.items.items().map(({ key }) => key),
            (keys, before) => {
                // leave a selected tab that is there, or never was
                const selected = untrack(value);
                const at = selected === undefined ? -1 : (before?.indexOf(selected) ?? -1);
                if (selected === undefined || at === -1 || keys.includes(selected)) {
                    return;
                }

                // take the next one a person can select, else the one before
                const near = [...keys.slice(at), ...keys.slice(0, at).toReversed()];
                const { delegate } = this.list;
                const next = untrack(() => near.find((key) => delegate.has(key))) ?? near[0];
                if (next !== undefined) {
                    this.select(next);
                }
            },
        );
    }

    /** List what handles the actions on tabs, read as they run, for an element's `ref`. */
    static handlers(
        tabs: () => TabsControl | undefined,
        direction: () => Direction,
    ): ((element: Element) => void)[] {
        // handle an action on the tab a person is on, or the selected one
        const onTab = (
            action: Action,
            isEnabled: (control: TabsControl, value: string) => boolean,
            run: (control: TabsControl, value: string) => void,
        ) =>
            handle(action, {
                isEnabled: (element) => {
                    const control = tabs();
                    const value = control?.valueAt(element);

                    return (
                        control !== undefined && value !== undefined && isEnabled(control, value)
                    );
                },
                run: ({ element }) => {
                    const control = untrack(tabs);
                    const value = untrack(() => control?.valueAt(element));
                    if (control !== undefined && value !== undefined) {
                        untrack(() => run(control, value));
                    }
                },
            });

        // move a tab to a side
        const sideways = (by: -1 | 1): number => (direction() === "rtl" ? -by : by);
        const onMove = (action: Action, by: -1 | 1) =>
            onTab(
                action,
                (control, value) => control.canMove(value, sideways(by)),
                (control, value) => control.move(value, sideways(by)),
            );
        const onClose = (action: Action, place: "all" | "others" | "after", least: number) =>
            onTab(
                action,
                (control, value) => control.closable(place, value).length >= least,
                (control, value) => {
                    for (const each of control.closable(place, value)) {
                        control.close(each);
                    }
                },
            );

        return [
            onTab(
                closeTab,
                (control, value) => control.isClosable(value),
                (control, value) => control.close(value),
            ),
            onClose(closeOtherTabs, "others", 1),
            onClose(closeTabsAfter, "after", 1),
            onClose(closeAllTabs, "all", 2),
            handle(reopenTab, {
                isEnabled: () => tabs()?.properties.onReopen !== undefined,
                run: () => tabs()?.properties.onReopen?.(),
            }),
            onTab(
                nextTab,
                (control) => control.values().length > 1,
                (control) => control.step(1),
            ),
            onTab(
                previousTab,
                (control) => control.values().length > 1,
                (control) => control.step(-1),
            ),
            handle(goToTab, {
                isEnabled: () => (tabs()?.values().length ?? 0) > 1,
                choices: () =>
                    untrack(() =>
                        (tabs()?.values() ?? []).map((value) => ({
                            value,
                            title: tabs()?.element(value)?.textContent?.trim() ?? value,
                        })),
                    ),
                run: ({ values }) => tabs()?.select(values.tab),
            }),
            onMove(moveTabLeft, -1),
            onMove(moveTabRight, 1),
        ];
    }

    /** List the values of the tabs in order. */
    values(): readonly string[] {
        return this.list.items.items().map(({ key }) => key);
    }

    /** Read the value of the tab an element is in, the selected one elsewhere. */
    valueAt(element: Element | null): string | undefined {
        const tab = element?.closest("[role=tab], [data-tab]");

        return tab?.getAttribute("data-value") ?? tab?.getAttribute("data-tab") ?? this.value();
    }

    /** Report whether a tab has a close button. */
    isClosable(value: string): boolean {
        return this.items.item(value)?.isClosable() === true;
    }

    /** List the tabs that close besides one, or after it, or all of them. */
    closable(place: "all" | "others" | "after", value: string): readonly string[] {
        const values = this.values();
        const at = values.indexOf(value);

        return values.filter(
            (each, index) =>
                this.isClosable(each) &&
                (place === "all" || (place === "others" ? index !== at : index > at)),
        );
    }

    /** Select the tab some places on, around the ends. */
    step(by: number): void {
        // find the place around the ends
        const values = this.values();
        const at = values.indexOf(this.value() ?? "");
        const next = values[(at + by + values.length) % values.length];
        if (next !== undefined) {
            this.select(next);
        }
    }

    /** Report whether a tab moves some places on among the others. */
    canMove(value: string, by: number): boolean {
        const values = this.values();
        const at = values.indexOf(value) + by;

        return this.properties.onMove !== undefined && at >= 0 && at < values.length;
    }

    /** Ask the owner to move a tab some places on, the focus staying on it. */
    move(value: string, by: number): void {
        // moving its element may drop the focus
        const isFocused = this.list.focus.isFocused();
        this.properties.onMove?.(value, this.values().indexOf(value) + by);
        flush();
        if (isFocused) {
            this.tab(value)?.focus();
        }
    }

    /** Ask the owner to close a tab, unless it has no close button. */
    close(value: string): void {
        if (untrack(() => this.isClosable(value))) {
            this.properties.onClose?.(value);
        }
    }

    /** Read the element of a tab itself. */
    tab(value: string): HTMLElement | undefined {
        return this.list.items
            .items()
            .find(({ key }) => key === value)
            ?.element();
    }

    /** Read the element of a tab, the item around it where it has one. */
    element(value: string): HTMLElement | undefined {
        return this.items.item(value)?.element() ?? this.tab(value);
    }

    /** Select a tab and tell the change handler. */
    select(value: string): void {
        this.#setValue(value);
    }

    /** Build the id of a tab. */
    tabId(value: string): string {
        return `${this.#prefix}-tab-${encodeURIComponent(value)}`;
    }

    /** Build the id of a tab's panel. */
    panelId(value: string): string {
        return `${this.#prefix}-panel-${encodeURIComponent(value)}`;
    }
}

/** Read the tabs of the nearest tabs root, refusing elements outside one. */
export function useTabs(): TabsControl {
    const control = useContext(TabsContext);
    if (control === null) {
        throw new TypeError("tab elements need a tabs root around them");
    }

    return control;
}

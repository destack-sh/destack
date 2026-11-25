import { type Direction, type Localization, t } from "@destack/locale";
import type { Action } from "@destack/object";
import { handle, untrack } from "@destack/view";
import {
    closePane,
    dockPane,
    evenPanes,
    floatTab,
    focusPaneDown,
    focusPaneLeft,
    focusPaneRight,
    focusPaneUp,
    maximizePane,
    moveTabTo,
    openWindow,
    splitDown,
    splitRight,
} from "./actions.ts";
import type { PaneGroupControl, PaneWindow } from "./control.ts";
import { PANE_TYPE, TAB_TYPE } from "./drag.ts";
import {
    type PaneBox,
    type PaneItem,
    PaneLayout,
    type PanePlace,
    type PaneSide,
} from "./layout.ts";

/** The sides a tab moves to in a pane of its own, as a choice writes them. */
const SIDES: readonly PaneSide[] = ["start", "end", "top", "bottom"];

/** Where the first float goes, about the middle and half the size of the group. */
const FLOAT_BOUNDS: PaneBox = { start: 0.25, top: 0.15, width: 0.5, height: 0.6 };

/** The step from a float to the next. */
const FLOAT_STEP = 0.04;

/** The floats before one goes where the first one did, as the steps then reach the sides. */
const FLOAT_COUNT = 6;

/** List what handles the actions on panes, for their group's `ref`. */
export function handlePanes(
    group: PaneGroupControl,
    locale: Localization,
): ((element: Element) => void)[] {
    // handle an action on the pane a person is in
    const direction = (): Direction => locale.direction;
    const onPane = (
        action: Action,
        isEnabled: (pane: string) => boolean,
        run: (pane: string, element: Element | null) => void,
        isOn?: (pane: string) => boolean,
    ) =>
        handle(action, {
            isEnabled: (element) => {
                const pane = group.paneAt(element);

                return pane !== undefined && !group.isLocked() && isEnabled(pane);
            },
            run: ({ element }) => {
                const pane = untrack(() => group.paneAt(element));
                if (pane !== undefined) {
                    untrack(() => run(pane, element));
                }
            },
            ...(isOn === undefined ? {} : { isOn: (element) => isOn(group.paneAt(element) ?? "") }),
        });

    // read the tab an element is on, or the selected one of its pane
    const itemAt = (element: Element | null): PaneItem | undefined => {
        // take the one the element is in, which another pane may have open
        const pane = PaneLayout.pane(group.layout(), group.paneAt(element) ?? "");
        const id = element?.closest("[data-tab]")?.getAttribute("data-tab");
        const on = group.item(id ?? "");

        return on ?? (pane === undefined ? undefined : PaneLayout.selected(pane));
    };

    // move the tab an element is on, or a copy of it
    const moveTab = (
        element: Element | null,
        place: PanePlace | PaneWindow,
        isCopied = false,
    ): void => {
        const item = itemAt(element);
        if (item !== undefined) {
            group.move({ id: item.id, type: TAB_TYPE, place, isCopied, items: [] });
        }
    };
    const split = (side: PaneSide) => (pane: string, element: Element | null) =>
        moveTab(element, { kind: "side", pane, side }, true);
    const right = (): PaneSide => (direction() === "rtl" ? "start" : "end");
    const left = (): PaneSide => (direction() === "rtl" ? "end" : "start");
    const hasItem = (pane: string): boolean =>
        (PaneLayout.pane(group.layout(), pane)?.items.length ?? 0) > 0;
    const hasSeveral = (): boolean => PaneLayout.panes(group.layout()).length > 1;

    // go to the pane at a side
    const go = (action: Action, side: () => PaneSide) =>
        onPane(
            action,
            (pane) => PaneLayout.neighbor(group.layout(), pane, side()) !== undefined,
            (pane) => {
                const next = PaneLayout.neighbor(group.layout(), pane, side());
                if (next !== undefined) {
                    group.change((layout) => PaneLayout.activate(layout, next.id));
                    group.focus(next.id);
                }
            },
        );

    // list where a tab can go: the other panes, and a pane of its own at each side
    const places = (element: Element | null): readonly { value: string; title: string }[] => {
        // leave out the pane it is in
        const own = group.paneAt(element);
        const panes = PaneLayout.panes(group.layout());
        const titles: Record<PaneSide, string> = {
            start: locale.render(t`A pane before this one`),
            end: locale.render(t`A pane after this one`),
            top: locale.render(t`A pane above this one`),
            bottom: locale.render(t`A pane below this one`),
        };
        const others = panes.flatMap((pane, index) =>
            pane.id === own ? [] : [{ value: pane.id, title: locale.render(t`Pane ${index + 1}`) }],
        );
        const count = PaneLayout.pane(group.layout(), own ?? "")?.items.length ?? 0;

        return count < 2
            ? others
            : [...others, ...SIDES.map((side) => ({ value: `side:${side}`, title: titles[side] }))];
    };

    return [
        onPane(splitRight, hasItem, (pane, element) => split(right())(pane, element)),
        onPane(splitDown, hasItem, split("bottom")),
        handle(moveTabTo, {
            isEnabled: (element) =>
                !group.isLocked() && itemAt(element) !== undefined && places(element).length > 0,
            choices: (_input, element) => untrack(() => places(element)),
            run: ({ element, values }) => {
                // move the tab to the pane, or to one of its own
                const pane = untrack(() => group.paneAt(element));
                const side = SIDES.find((each) => values.place === `side:${each}`);
                untrack(() =>
                    moveTab(
                        element,
                        side === undefined || pane === undefined
                            ? { kind: "item", pane: values.place }
                            : { kind: "side", pane, side },
                    ),
                );
            },
        }),
        onPane(
            floatTab,
            (pane) =>
                hasItem(pane) &&
                (PaneLayout.floatOf(group.layout(), pane) === undefined ||
                    (PaneLayout.pane(group.layout(), pane)?.items.length ?? 0) > 1),
            // float it a step from the last one
            (_pane, element) =>
                moveTab(element, { kind: "float", bounds: freeBounds(group.layout()) }),
        ),
        onPane(
            dockPane,
            (pane) => PaneLayout.floatOf(group.layout(), pane) !== undefined,
            (pane) =>
                group.move({
                    id: pane,
                    type: PANE_TYPE,
                    place: { kind: "side", side: "end" },
                    isCopied: false,
                    items: [],
                }),
        ),
        onPane(
            openWindow,
            (pane) => {
                const current = PaneLayout.pane(group.layout(), pane);
                const item = current === undefined ? undefined : PaneLayout.selected(current);

                return item !== undefined && group.canOpen(item.id);
            },
            (_pane, element) => moveTab(element, { kind: "window" }),
        ),
        go(focusPaneLeft, left),
        go(focusPaneRight, right),
        go(focusPaneUp, () => "top"),
        go(focusPaneDown, () => "bottom"),
        onPane(
            maximizePane,
            () => hasSeveral() || group.layout().maximized !== undefined,
            (pane) => group.toggleMaximized(pane),
            (pane) => group.layout().maximized === pane,
        ),
        onPane(closePane, hasSeveral, (pane) => group.remove(pane)),
        onPane(evenPanes, hasSeveral, () =>
            group.change((layout) => PaneLayout.distribute(layout)),
        ),
    ];
}

/** Read where the next float goes, a step from the last. */
function freeBounds(layout: PaneLayout): PaneBox {
    const step = (layout.floats.length % FLOAT_COUNT) * FLOAT_STEP;

    return { ...FLOAT_BOUNDS, start: FLOAT_BOUNDS.start + step, top: FLOAT_BOUNDS.top + step };
}

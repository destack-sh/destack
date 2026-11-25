import { type Direction, type Localization, t } from "@destack/locale";
import type { Action } from "@destack/object";
import { handle, untrack } from "@destack/view";
import {
    closePane,
    evenPanes,
    focusPaneDown,
    focusPaneLeft,
    focusPaneRight,
    focusPaneUp,
    maximizePane,
    moveTabTo,
    splitDown,
    splitRight,
} from "./actions.ts";
import { type PaneGroupControl } from "./control.ts";
import { TAB_TYPE } from "./drag.ts";
import { type PaneItem, PaneLayout, type PanePlace, type PaneSide } from "./layout.ts";

/** The sides a tab moves to in a pane of its own, as a choice writes them. */
const SIDES: readonly PaneSide[] = ["start", "end", "top", "bottom"];

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
        // take the one the element is in
        const pane = PaneLayout.pane(group.layout(), group.paneAt(element) ?? "");
        const id = element?.closest("[data-tab]")?.getAttribute("data-tab");
        const on = pane?.items.find((item) => item.id === id);

        return on ?? (pane === undefined ? undefined : PaneLayout.selected(pane));
    };

    // open a copy of a tab beside its pane
    const split = (side: PaneSide) => (pane: string, element: Element | null) => {
        const item = itemAt(element);
        if (item !== undefined) {
            group.move({
                id: item.id,
                type: TAB_TYPE,
                place: { kind: "side", pane, side },
                isCopied: true,
            });
        }
    };
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
                const item = untrack(() => itemAt(element));
                const pane = untrack(() => group.paneAt(element));
                const side = SIDES.find((each) => values.place === `side:${each}`);
                if (item === undefined || pane === undefined) {
                    return;
                }
                const place: PanePlace =
                    side === undefined
                        ? { kind: "item", pane: values.place }
                        : { kind: "side", pane, side };
                group.move({ id: item.id, type: TAB_TYPE, place, isCopied: false });
            },
        }),
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

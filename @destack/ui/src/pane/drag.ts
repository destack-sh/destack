import { t } from "@destack/locale";
import { untrack, useLocale } from "@destack/view";
import { Draggable, type DragOperation, type DragTarget, Droppable } from "../drag/index.ts";
import type { PaneEntry, PaneGroupControl } from "./control.ts";
import { PaneLayout, type PanePlace, type PaneSide } from "./layout.ts";

/** The type of a tab as it is dragged. */
export const TAB_TYPE = "tab";

/** The type of a pane as it is dragged. */
export const PANE_TYPE = "pane";

/** The types a pane takes unless it says otherwise. */
const TYPES: readonly string[] = [TAB_TYPE, PANE_TYPE];

/** The share of a pane's body each of its sides takes drops in, as editors draw it. */
const SIDE_SHARE = 0.25;

/** The pixels along the group's own sides that take drops beside every pane. */
const GROUP_REACH = 16;

/** The drags of a group's tabs and panes, and where they land. */
export class PaneDrag {
    /** The group. */
    readonly group: PaneGroupControl;

    /** Take what a person drags along the group's own sides. */
    constructor(group: PaneGroupControl) {
        // take drops beside every pane
        this.group = group;
        const locale = useLocale();
        void new Droppable(
            {
                id: this.group.prefix,
                group: this.group.prefix,
                axis: "both",
                element: () => this.group.element(),
                accepts: (draggable) => !this.group.isLocked() && TYPES.includes(draggable.type),
                measure: () => undefined,
                targetAt: () => undefined,
                step: () => undefined,
                enter: () => ({ kind: "side", droppable: this.group.prefix, side: "end" }),
                describe: (target) => ({
                    kind: "label",
                    label: locale.render(sideOf(target)),
                }),
                drop: (operation) => {
                    const place = placeOf(operation.target, undefined);
                    if (place !== undefined) {
                        this.#drop(operation, place);
                    }
                },
            },
            group.manager,
        );
    }

    /** Take what a person drags over a pane, on its sides and its middle. */
    add(entry: PaneEntry): void {
        // say where a drop lands
        const locale = useLocale();
        const droppable = `${this.group.prefix}-${entry.id}`;

        // take drops over its body
        void new Droppable(
            {
                id: droppable,
                group: this.group.prefix,
                axis: "both",
                element: entry.element,
                accepts: (draggable) =>
                    !this.group.isLocked() && (entry.accepts() ?? TYPES).includes(draggable.type),
                measure: () => undefined,
                targetAt: (x, y, operation) => this.#targetAt(entry, droppable, x, y, operation),
                step: () => undefined,
                enter: () => ({ kind: "side", droppable, side: "middle" }),
                describe: (target) => ({
                    kind: "label",
                    label: locale.render(sideOf(target)),
                }),
                drop: (operation) => {
                    const place = placeOf(operation.target, entry.id);
                    if (place !== undefined) {
                        this.#drop(operation, place);
                    }
                },
            },
            this.group.manager,
        );
    }

    /** Make a draggable of a pane, which its bar lifts. */
    draggable(pane: string, element: () => HTMLElement | undefined): Draggable {
        const locale = useLocale();

        return new Draggable(
            {
                id: `${this.group.prefix}-drag-${pane}`,
                item: pane,
                type: PANE_TYPE,
                kind: "move",
                label: () => locale.render(t`the pane`),
                element,
                home: () => ({
                    kind: "side",
                    droppable: `${this.group.prefix}-${pane}`,
                    side: "middle",
                }),
                isDisabled: () => this.group.isLocked(),
                // leave presses on its tabs to them
                canLift: ({ target }) =>
                    !(target instanceof Element) || target.closest("[role=tab]") === null,
            },
            this.group.manager,
        );
    }

    /** Read where a drag aims: the place, and the box it lands in. */
    aim(): { readonly pane: string | undefined; readonly side: PaneSide | "middle" } | undefined {
        // read the aim inside this group
        const target = this.group.manager.operation()?.target;
        if (target?.kind !== "side") {
            return undefined;
        } else if (target.droppable === this.group.prefix) {
            return { pane: undefined, side: target.side };
        }
        const placement = this.group
            .placements()
            .find(({ pane }) => `${this.group.prefix}-${pane.id}` === target.droppable);

        return placement === undefined ? undefined : { pane: placement.pane.id, side: target.side };
    }

    /** Find where a point lands in a pane. */
    #targetAt(
        entry: PaneEntry,
        droppable: string,
        x: number,
        y: number,
        operation: DragOperation,
    ): DragTarget | undefined {
        // take what comes near the group's own sides beside every pane
        const isBackward = this.group.manager.locale.direction === "rtl";
        const side = (physical: PaneSide): PaneSide =>
            !isBackward || physical === "top" || physical === "bottom"
                ? physical
                : physical === "start"
                  ? "end"
                  : "start";
        const layout = untrack(() => this.group.options.layout);
        const group = this.group.element()?.getBoundingClientRect();
        if (group !== undefined && layout.root?.kind === "split") {
            const [outer, distance] = nearestSide(group, x, y, (length) => length);
            if (distance <= GROUP_REACH) {
                return { kind: "side", droppable: this.group.prefix, side: side(outer) };
            }
        }

        // take a pane's sides and its middle, where that changes something
        const bounds = (entry.body() ?? entry.element())?.getBoundingClientRect();
        if (bounds === undefined || y < bounds.top) {
            return undefined;
        }
        const [near, share] = nearestSide(bounds, x, y, (length, whole) => length / whole);
        const found = share <= SIDE_SHARE ? side(near) : "middle";
        const pane = PaneLayout.pane(layout, entry.id);
        const isOwn =
            operation.type === PANE_TYPE
                ? operation.item === entry.id
                : pane?.items.some(({ id }) => id === operation.item) === true;
        const isAlone = isOwn && (operation.type === PANE_TYPE || pane?.items.length === 1);

        return isAlone || (isOwn && found === "middle")
            ? undefined
            : { kind: "side", droppable, side: found };
    }

    /** Make the move of a drop. */
    #drop(operation: DragOperation, place: PanePlace): void {
        this.group.move({
            id: operation.item,
            type: operation.type,
            place,
            isCopied: operation.effect === "copy",
        });
    }
}

/** Read the side of a box a point is nearest, and how near by a measure of its own. */
function nearestSide(
    box: DOMRect,
    x: number,
    y: number,
    measure: (length: number, whole: number) => number,
): readonly [PaneSide, number] {
    const distances: readonly (readonly [PaneSide, number])[] = [
        ["start", measure(x - box.left, box.width)],
        ["end", measure(box.right - x, box.width)],
        ["top", measure(y - box.top, box.height)],
        ["bottom", measure(box.bottom - y, box.height)],
    ];

    return distances.reduce((least, each) => (each[1] < least[1] ? each : least));
}

/** Read the place of a target. */
function placeOf(target: DragTarget, pane: string | undefined): PanePlace | undefined {
    // land among a pane's items, or beside it, or beside all
    if (target.kind !== "side") {
        return undefined;
    } else if (target.side !== "middle") {
        return pane === undefined
            ? { kind: "side", side: target.side }
            : { kind: "side", pane, side: target.side };
    }

    return pane === undefined ? undefined : { kind: "item", pane };
}

/** Name the side a drag aims at. */
function sideOf(target: DragTarget) {
    // name each side, or the pane itself
    const side = target.kind === "side" ? target.side : "middle";
    if (side === "start") {
        return t`Before the pane`;
    } else if (side === "end") {
        return t`After the pane`;
    } else if (side === "top") {
        return t`Above the pane`;
    } else if (side === "bottom") {
        return t`Below the pane`;
    }

    return t`Into the pane`;
}

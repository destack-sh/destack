import type { Axis } from "../position/index.ts";

/** A side of a box, in the direction of the text. */
export type PaneSide = "start" | "end" | "top" | "bottom";

/** Something open in a pane, as one of its tabs. */
export interface PaneItem {
    /** The id, unique in the layout. */
    readonly id: string;
    /** What it shows, which several items may share. */
    readonly value: string;
}

/** A place that shows one of its items. */
export interface PaneLeaf {
    /** A pane, where the tree ends. */
    readonly kind: "leaf";
    /** The id, unique in the layout. */
    readonly id: string;
    /** The items in order. */
    readonly items: readonly PaneItem[];
    /** The ids of the items as they were last selected, the selected one last. */
    readonly recent: readonly string[];
    /** Whether it stays without items. */
    readonly isKept: boolean;
}

/** Panes and further splits side by side. */
export interface PaneSplit {
    /** A split, around two parts or more. */
    readonly kind: "split";
    /** The axis its parts line up along. */
    readonly direction: Axis;
    /** The parts in order, none of them a split along the same axis. */
    readonly children: readonly PaneNode[];
    /** The parts' shares, which add up to one. */
    readonly sizes: readonly number[];
}

/** A pane, or a split of them. */
export type PaneNode = PaneLeaf | PaneSplit;

/** Panes that float over the others. */
export interface PaneFloat {
    /** The id, unique in the layout. */
    readonly id: string;
    /** The panes. */
    readonly root: PaneNode;
    /** Where it is, as shares of the group's size. */
    readonly bounds: PaneBox;
}

/** Where each pane is, and what it has open. */
export interface PaneLayout {
    /** The panes side by side, undefined without any. */
    readonly root: PaneNode | undefined;
    /** The panes that float over them, the one on top last. */
    readonly floats: readonly PaneFloat[];
    /** The id of the pane the person is in, undefined without panes. */
    readonly active: string | undefined;
    /** The id of the pane that covers the others, undefined while none does. */
    readonly maximized: string | undefined;
}

/** A tree of panes: those side by side, or a float's. */
export interface PaneTree {
    /** The panes. */
    readonly root: PaneNode;
    /** The id of the float, undefined side by side. */
    readonly float: string | undefined;
    /** Where it is. */
    readonly bounds: PaneBox;
    /** How high it floats, zero side by side. */
    readonly height: number;
}

/** Where an item or a pane goes. */
export type PanePlace =
    | {
          /** Among a pane's items. */
          readonly kind: "item";
          /** The pane's id. */
          readonly pane: string;
          /** The place among its other items, its end by default. */
          readonly index?: number;
      }
    | {
          /** Beside a pane, or beside all of them. */
          readonly kind: "side";
          /** The pane's id, every pane by default. */
          readonly pane?: string;
          /** The side. */
          readonly side: PaneSide;
      }
    | {
          /** In a pane that floats over the others. */
          readonly kind: "float";
          /** Where it floats. */
          readonly bounds: PaneBox;
      };

/** A box as shares of the group's size. */
export interface PaneBox {
    /** The share before it along the text. */
    readonly start: number;
    /** The share above it. */
    readonly top: number;
    /** The share it is wide. */
    readonly width: number;
    /** The share it is high. */
    readonly height: number;
}

/** Where a pane is. */
export interface PanePlacement {
    /** The pane. */
    readonly pane: PaneLeaf;
    /** Its box. */
    readonly box: PaneBox;
    /** How high it floats, zero for one side by side with the others. */
    readonly height: number;
}

/** Where a split is in a layout. */
export interface PanePath {
    /** The id of the float it is in, undefined side by side. */
    readonly float: string | undefined;
    /** The places of the splits down to it, from the root. */
    readonly path: readonly number[];
}

/** The line between two parts of a split. */
export interface PaneEdge extends PanePath {
    /** The places of the splits down to its own, from the root. */
    readonly path: readonly number[];
    /** The place of the part before it. */
    readonly index: number;
    /** The axis its parts line up along. */
    readonly direction: Axis;
    /** The box of both parts together. */
    readonly box: PaneBox;
    /** The share of that box the part before it takes. */
    readonly share: number;
}

/** The whole of a group. */
const WHOLE: PaneBox = { start: 0, top: 0, width: 1, height: 1 };

/** The verbs of layouts, each of which returns another layout. */
export const PaneLayout = {
    /** Lay out panes in a row, each with its items. */
    of(...panes: readonly (readonly (string | PaneItem)[])[]): PaneLayout {
        // select the first item of each, and activate the first pane
        const leaves = panes.flatMap((given, index): PaneLeaf[] => {
            const items = given.map(itemOf);
            const [first] = items;

            return first === undefined
                ? []
                : [
                      {
                          kind: "leaf",
                          id: `pane-${String(index + 1)}`,
                          items,
                          recent: [first.id],
                          isKept: false,
                      },
                  ];
        });
        const [only] = leaves;
        const row: PaneSplit = {
            kind: "split",
            direction: "horizontal",
            children: leaves,
            sizes: evenSizes(leaves.length),
        };

        return {
            root: leaves.length > 1 ? row : only,
            floats: [],
            active: only?.id,
            maximized: undefined,
        };
    },

    /** List the trees of panes, the one on top last. */
    trees(layout: PaneLayout): readonly PaneTree[] {
        // put those side by side under the floats
        const floats = layout.floats.map(({ id, root, bounds }, index) => ({
            root,
            float: id,
            bounds,
            height: index + 1,
        }));

        return layout.root === undefined
            ? floats
            : [{ root: layout.root, float: undefined, bounds: WHOLE, height: 0 }, ...floats];
    },

    /** List the panes in the order they are read. */
    panes(layout: PaneLayout): readonly PaneLeaf[] {
        return PaneLayout.trees(layout).flatMap(({ root }) => panesOf(root));
    },

    /** Read a pane by its id. */
    pane(layout: PaneLayout, id: string): PaneLeaf | undefined {
        return PaneLayout.panes(layout).find((pane) => pane.id === id);
    },

    /** Read the pane an item is open in. */
    paneOf(layout: PaneLayout, item: string): PaneLeaf | undefined {
        return PaneLayout.panes(layout).find((pane) => pane.items.some(({ id }) => id === item));
    },

    /** Read the pane the person is in. */
    active(layout: PaneLayout): PaneLeaf | undefined {
        return layout.active === undefined ? undefined : PaneLayout.pane(layout, layout.active);
    },

    /** Read the selected item of a pane. */
    selected(pane: PaneLeaf): PaneItem | undefined {
        return pane.items.find(({ id }) => id === pane.recent.at(-1));
    },

    /** Read an id for another item of a value, which no item has. */
    freeItem(layout: PaneLayout, value: string): string {
        // take the value itself, else the first number that is free
        const taken = new Set(
            PaneLayout.panes(layout).flatMap(({ items }) => items.map(({ id }) => id)),
        );
        let number = 1;
        while (taken.has(number === 1 ? value : `${value}-${String(number)}`)) {
            number += 1;
        }

        return number === 1 ? value : `${value}-${String(number)}`;
    },

    /** Open an item and select it, in the active pane without a place or at one that is gone. */
    open(layout: PaneLayout, item: string | PaneItem, place?: PanePlace): PaneLayout {
        // select one that is open there already, beside nothing
        const opened = itemOf(item);
        const given = place !== undefined && isThere(layout, place) ? place : undefined;
        const target = given ?? placeIn(layout);
        const standing = PaneLayout.paneOf(layout, opened.id);
        if (standing !== undefined && given === undefined) {
            return PaneLayout.select(layout, opened.id);
        }
        // move one that is open elsewhere
        else if (standing !== undefined && target !== undefined) {
            return PaneLayout.move(layout, opened.id, target);
        }
        // add it to a pane
        else if (target?.kind === "item") {
            return insert(layout, target, opened);
        }

        // add it in a pane of its own
        return apart(layout, opened, target);
    },

    /** Select an item, in its pane and its pane among the others. */
    select(layout: PaneLayout, item: string): PaneLayout {
        // leave a layout without it
        const pane = PaneLayout.paneOf(layout, item);
        if (pane === undefined) {
            return layout;
        }
        const recent = [...pane.recent.filter((each) => each !== item), item];

        return PaneLayout.activate(replace(layout, pane.id, { ...pane, recent }), pane.id);
    },

    /** Activate a pane, which then covers the others where one did, its float on top. */
    activate(layout: PaneLayout, pane: string): PaneLayout {
        // leave a layout without it
        if (PaneLayout.pane(layout, pane) === undefined) {
            return layout;
        }
        const maximized = layout.maximized === undefined ? undefined : pane;
        const over = PaneLayout.floatOf(layout, pane);
        const floats =
            over === undefined || over === layout.floats.at(-1)
                ? layout.floats
                : [...layout.floats.filter((each) => each !== over), over];

        return { ...layout, floats, active: pane, maximized };
    },

    /** Move an item and select it where it lands. */
    move(layout: PaneLayout, item: string, place: PanePlace): PaneLayout {
        // leave an item that is not open, a place that is gone, and a pane's only item beside it
        const from = PaneLayout.paneOf(layout, item);
        const moved = from?.items.find(({ id }) => id === item);
        const isAlone = from?.items.length === 1 && !from.isKept;
        if (
            from === undefined ||
            moved === undefined ||
            !isThere(layout, place) ||
            (place.kind === "side" && isAlone && place.pane === from.id)
        ) {
            return layout;
        }

        // take it out, its pane standing until it landed
        const without = replace(layout, from.id, withoutItem(from, item));
        const landed =
            place.kind === "item" ? insert(without, place, moved) : apart(without, moved, place);

        return settle(landed);
    },

    /** Close an item, and its pane with its last one unless it stays. */
    close(layout: PaneLayout, item: string): PaneLayout {
        // leave a layout without it
        const pane = PaneLayout.paneOf(layout, item);
        if (pane === undefined) {
            return layout;
        }

        return settle(replace(layout, pane.id, withoutItem(pane, item)));
    },

    /** Close a pane with every item in it. */
    remove(layout: PaneLayout, pane: string): PaneLayout {
        return settle(replace(layout, pane, undefined));
    },

    /** Move every item of a pane to the end of another's, and close it. */
    merge(layout: PaneLayout, pane: string, into: string): PaneLayout {
        // leave a pane by itself
        const from = PaneLayout.pane(layout, pane);
        const target = PaneLayout.pane(layout, into);
        if (from === undefined || target === undefined || pane === into) {
            return layout;
        }

        // select what was selected
        const merged: PaneLeaf = {
            ...target,
            items: [...target.items, ...from.items],
            recent: [...target.recent, ...from.recent],
        };

        return PaneLayout.activate(
            settle(replace(replace(layout, into, merged), pane, undefined)),
            into,
        );
    },

    /** Move a pane with its items beside another, or beside all. */
    dock(layout: PaneLayout, pane: string, place: PanePlace): PaneLayout {
        // merge it into another's items
        const from = PaneLayout.pane(layout, pane);
        if (place.kind === "item") {
            return PaneLayout.merge(layout, pane, place.pane);
        }
        // leave one beside itself, or beside one that is gone
        else if (from === undefined || !isThere(layout, place)) {
            return layout;
        } else if (place.kind === "side" && place.pane === pane) {
            return layout;
        }
        const without = settle(replace(layout, pane, undefined));

        return PaneLayout.activate(put(without, from, place), pane);
    },

    /** Exchange the places of two panes. */
    swap(layout: PaneLayout, one: string, other: string): PaneLayout {
        // leave panes that are not there
        const first = PaneLayout.pane(layout, one);
        const second = PaneLayout.pane(layout, other);
        if (first === undefined || second === undefined) {
            return layout;
        }

        return mapTrees(layout, (root) =>
            mapPanes(root, (pane) => (pane.id === one ? second : pane.id === other ? first : pane)),
        );
    },

    /** Cover the others with a pane. */
    maximize(layout: PaneLayout, pane: string): PaneLayout {
        // leave a layout without it
        return PaneLayout.pane(layout, pane) === undefined
            ? layout
            : { ...layout, active: pane, maximized: pane };
    },

    /** Uncover the others. */
    restore(layout: PaneLayout): PaneLayout {
        return { ...layout, maximized: undefined };
    },

    /** Set where a float is. */
    place(layout: PaneLayout, float: string, bounds: PaneBox): PaneLayout {
        return {
            ...layout,
            floats: layout.floats.map((each) => (each.id === float ? { ...each, bounds } : each)),
        };
    },

    /** Set the shares of the parts of a split. */
    resize(layout: PaneLayout, at: PanePath, sizes: readonly number[]): PaneLayout {
        return mapTrees(layout, (root, float) =>
            float === at.float ? resizeAt(root, at.path, sizes) : root,
        );
    },

    /** Give the parts of a split even shares, of every split by default. */
    distribute(layout: PaneLayout, at?: PanePath): PaneLayout {
        // even out the parts of each split
        if (at === undefined) {
            return mapTrees(layout, evenOut);
        }
        const split = PaneLayout.node(layout, at);

        return split?.kind === "split"
            ? PaneLayout.resize(layout, at, evenSizes(split.children.length))
            : layout;
    },

    /** Read the node at a path. */
    node(layout: PaneLayout, at: PanePath): PaneNode | undefined {
        const tree = PaneLayout.trees(layout).find(({ float }) => float === at.float);

        return tree === undefined ? undefined : nodeAt(tree.root, at.path);
    },

    /** Read the float a pane is in, undefined side by side. */
    floatOf(layout: PaneLayout, pane: string): PaneFloat | undefined {
        return layout.floats.find(({ root }) => panesOf(root).some(({ id }) => id === pane));
    },

    /** Read where each pane is, only the one that covers the others. */
    placements(layout: PaneLayout): readonly PanePlacement[] {
        // cover the others
        const maximized =
            layout.maximized === undefined ? undefined : PaneLayout.pane(layout, layout.maximized);
        if (maximized !== undefined) {
            return [{ pane: maximized, box: WHOLE, height: layout.floats.length }];
        }

        return PaneLayout.trees(layout).flatMap(({ root, bounds, height }) =>
            placementsOf(root, bounds).map((each) => ({ ...each, height })),
        );
    },

    /** Read the lines between the parts of every split. */
    edges(layout: PaneLayout): readonly PaneEdge[] {
        return layout.maximized === undefined
            ? PaneLayout.trees(layout).flatMap(({ root, bounds, float }) =>
                  edgesOf(root, bounds, { float, path: [] }),
              )
            : [];
    },

    /** Read the share a pane takes beside all the others. */
    shareBeside(layout: PaneLayout, side: PaneSide): number {
        // take an even part of a split along the axis, else a half
        const { root } = layout;
        const isAlong = root?.kind === "split" && root.direction === axisOf(side);

        return isAlong ? 1 / (root.children.length + 1) : 1 / 2;
    },

    /** Read the pane on a side of another, the nearest its middle. */
    neighbor(layout: PaneLayout, pane: string, side: PaneSide): PaneLeaf | undefined {
        // look across the side it shares with others
        const own = PaneLayout.floatOf(layout, pane)?.id;
        const tree = PaneLayout.trees(layout).find(({ float }) => float === own);
        const placements = tree === undefined ? [] : placementsOf(tree.root, tree.bounds);
        const from = placements.find((each) => each.pane.id === pane)?.box;
        if (from === undefined) {
            return undefined;
        }
        const isInline = side === "start" || side === "end";
        const near = (box: PaneBox): number =>
            isInline
                ? Math.abs(box.top + box.height / 2 - from.top - from.height / 2)
                : Math.abs(box.start + box.width / 2 - from.start - from.width / 2);
        const touching = placements.filter(({ box }) => {
            const isAlong = isInline
                ? box.top < from.top + from.height - EPSILON &&
                  box.top + box.height > from.top + EPSILON
                : box.start < from.start + from.width - EPSILON &&
                  box.start + box.width > from.start + EPSILON;
            const gap = {
                start: box.start + box.width - from.start,
                end: box.start - from.start - from.width,
                top: box.top + box.height - from.top,
                bottom: box.top - from.top - from.height,
            }[side];

            return isAlong && Math.abs(gap) < EPSILON;
        });

        return touching.toSorted((one, other) => near(one.box) - near(other.box))[0]?.pane;
    },
};

/** The share by which sums of shares round. */
const EPSILON = 1e-6;

/** Read an item, one whose value is its id from text. */
function itemOf(item: string | PaneItem): PaneItem {
    return typeof item === "string" ? { id: item, value: item } : item;
}

/** Take an item out of a pane, selecting the one selected before it, else the one beside it. */
function withoutItem(pane: PaneLeaf, item: string): PaneLeaf {
    // keep the order the others were selected in
    const at = pane.items.findIndex(({ id }) => id === item);
    const items = pane.items.filter(({ id }) => id !== item);
    const recent = pane.recent.filter((id) => id !== item);
    const near = items[Math.min(at, items.length - 1)];

    return {
        ...pane,
        items,
        recent: recent.length === 0 && near !== undefined ? [near.id] : recent,
    };
}

/** Read the axis across a side. */
function axisOf(side: PaneSide): Axis {
    return side === "start" || side === "end" ? "horizontal" : "vertical";
}

/** Read whether a side comes before what it is a side of. */
function isBefore(side: PaneSide): boolean {
    return side === "start" || side === "top";
}

/** Read even shares of a number of parts. */
function evenSizes(count: number): readonly number[] {
    return Array.from({ length: count }, () => 1 / count);
}

/** List the panes of a node in the order they are read. */
function panesOf(node: PaneNode): readonly PaneLeaf[] {
    return node.kind === "leaf" ? [node] : node.children.flatMap((child) => panesOf(child));
}

/** Read where an item opens without a place: the end of the active pane. */
function placeIn(layout: PaneLayout): PanePlace | undefined {
    const pane = PaneLayout.active(layout) ?? PaneLayout.panes(layout)[0];

    return pane === undefined ? undefined : { kind: "item", pane: pane.id };
}

/** Replace every tree, leaving out one without panes. */
function mapTrees(
    layout: PaneLayout,
    change: (root: PaneNode, float: string | undefined) => PaneNode | undefined,
): PaneLayout {
    return {
        ...layout,
        root: layout.root === undefined ? undefined : change(layout.root, undefined),
        floats: layout.floats.flatMap((float) => {
            const root = change(float.root, float.id);

            return root === undefined ? [] : [{ ...float, root }];
        }),
    };
}

/** Report whether the pane of a place is there. */
function isThere(layout: PaneLayout, place: PanePlace): boolean {
    return (
        place.kind === "float" ||
        place.pane === undefined ||
        PaneLayout.pane(layout, place.pane) !== undefined
    );
}

/** Read an id neither a pane nor a float has. */
function freeId(layout: PaneLayout, prefix: "pane" | "float"): string {
    // take the first number that is free
    const taken = new Set([
        ...PaneLayout.panes(layout).map(({ id }) => id),
        ...layout.floats.map(({ id }) => id),
    ]);
    let number = 1;
    while (taken.has(`${prefix}-${String(number)}`)) {
        number += 1;
    }

    return `${prefix}-${String(number)}`;
}

/** Insert an item among a pane's and select it. */
function insert(
    layout: PaneLayout,
    place: Extract<PanePlace, { kind: "item" }>,
    item: PaneItem,
): PaneLayout {
    // leave a layout without the pane
    const pane = PaneLayout.pane(layout, place.pane);
    if (pane === undefined) {
        return layout;
    }
    const items = pane.items.toSpliced(place.index ?? pane.items.length, 0, item);

    return PaneLayout.select(replace(layout, pane.id, { ...pane, items }), item.id);
}

/** Open an item in a pane of its own, beside another or all, or afloat. */
function apart(
    layout: PaneLayout,
    item: string | PaneItem,
    place?: Exclude<PanePlace, { kind: "item" }>,
): PaneLayout {
    // open at the end without a place
    const opened = itemOf(item);
    const id = freeId(layout, "pane");
    const pane: PaneLeaf = {
        kind: "leaf",
        id,
        items: [opened],
        recent: [opened.id],
        isKept: false,
    };
    const next = put(layout, pane, place ?? { kind: "side", side: "end" });

    return PaneLayout.activate(next, id);
}

/** Put a pane beside another or all, or afloat. */
function put(
    layout: PaneLayout,
    pane: PaneLeaf,
    place: Exclude<PanePlace, { kind: "item" }>,
): PaneLayout {
    // float it on top
    if (place.kind === "float") {
        const float = { id: freeId(layout, "float"), root: pane, bounds: place.bounds };

        return { ...layout, floats: [...layout.floats, float], maximized: undefined };
    }

    return splitAt(layout, pane, place);
}

/** Put a pane beside another, taking half of its room, or beside all, taking an even part. */
function splitAt(
    layout: PaneLayout,
    pane: PaneLeaf,
    place: Extract<PanePlace, { kind: "side" }>,
): PaneLayout {
    // go beside all of them
    const { root } = layout;
    const direction = axisOf(place.side);
    if (place.pane === undefined) {
        // take the whole without others, else an even part
        if (root === undefined) {
            return { ...layout, root: pane };
        }
        const parts =
            root.kind === "split" && root.direction === direction ? root.children : [root];
        const sizes = root.kind === "split" && root.direction === direction ? root.sizes : [1];
        const share = 1 / (parts.length + 1);
        const rest = sizes.map((size) => size * (1 - share));

        return {
            ...layout,
            root: {
                kind: "split",
                direction,
                children: isBefore(place.side) ? [pane, ...parts] : [...parts, pane],
                sizes: isBefore(place.side) ? [share, ...rest] : [...rest, share],
            },
        };
    }

    // take half of the other's room
    const target = place.pane;
    const pair = (other: PaneLeaf): PaneSplit => ({
        kind: "split",
        direction,
        children: isBefore(place.side) ? [pane, other] : [other, pane],
        sizes: [0.5, 0.5],
    });

    return mapTrees(layout, (tree) =>
        simplify(mapPanes(tree, (each) => (each.id === target ? pair(each) : each))),
    );
}

/** Replace every pane of a node. */
function mapPanes(node: PaneNode, change: (pane: PaneLeaf) => PaneNode): PaneNode {
    return node.kind === "leaf"
        ? change(node)
        : { ...node, children: node.children.map((child) => mapPanes(child, change)) };
}

/** Replace one pane, or leave it out. */
function replace(layout: PaneLayout, id: string, next: PaneLeaf | undefined): PaneLayout {
    // leave it out of its split, the others taking its room
    return next === undefined
        ? mapTrees(layout, (root) => filterPanes(root, (pane) => pane.id !== id))
        : mapTrees(layout, (root) => mapPanes(root, (pane) => (pane.id === id ? next : pane)));
}

/** Leave out panes, and splits without parts, the rest taking their room in proportion. */
function filterPanes(node: PaneNode, keeps: (pane: PaneLeaf) => boolean): PaneNode | undefined {
    // keep or drop a pane
    if (node.kind === "leaf") {
        return keeps(node) ? node : undefined;
    }

    // share out the room of what goes
    const parts = node.children.flatMap((child, index) => {
        const kept = filterPanes(child, keeps);

        return kept === undefined ? [] : [{ kept, size: node.sizes[index] ?? 0 }];
    });
    const total = parts.reduce((sum, { size }) => sum + size, 0);

    return parts.length === 0
        ? undefined
        : simplify({
              ...node,
              children: parts.map(({ kept }) => kept),
              sizes: parts.map(({ size }) => size / total),
          });
}

/** Unwrap splits of one part, and splits in a split along the same axis. */
function simplify(node: PaneNode): PaneNode {
    // leave a pane
    if (node.kind === "leaf") {
        return node;
    }

    // take the parts of a split along the same axis in its place
    const parts = node.children.flatMap((each, index) => {
        const child = simplify(each);
        const size = node.sizes[index] ?? 0;

        return child.kind === "split" && child.direction === node.direction
            ? child.children.map((inner, place) => ({
                  node: inner,
                  size: size * (child.sizes[place] ?? 0),
              }))
            : [{ node: child, size }];
    });
    const [only] = parts;

    return parts.length === 1 && only !== undefined
        ? only.node
        : {
              ...node,
              children: parts.map((part) => part.node),
              sizes: parts.map(({ size }) => size),
          };
}

/** Leave out panes without items, and activate one that is there. */
function settle(layout: PaneLayout): PaneLayout {
    // keep what has items, or stays without
    const kept = mapTrees(layout, (root) =>
        filterPanes(root, (pane) => pane.items.length > 0 || pane.isKept),
    );
    const panes = PaneLayout.panes(kept);
    const has = (id: string | undefined): boolean => panes.some((pane) => pane.id === id);

    return {
        ...kept,
        active: has(layout.active) ? layout.active : panes[0]?.id,
        maximized: has(layout.maximized) ? layout.maximized : undefined,
    };
}

/** Read the node at a path. */
function nodeAt(node: PaneNode, path: readonly number[]): PaneNode | undefined {
    const [index, ...rest] = path;
    const child = index === undefined || node.kind === "leaf" ? undefined : node.children[index];

    return index === undefined ? node : child === undefined ? undefined : nodeAt(child, rest);
}

/** Set the shares of the split at a path. */
function resizeAt(node: PaneNode, path: readonly number[], sizes: readonly number[]): PaneNode {
    // leave a pane
    const [index, ...rest] = path;
    if (node.kind === "leaf") {
        return node;
    } else if (index === undefined) {
        return { ...node, sizes };
    }

    return {
        ...node,
        children: node.children.map((child, place) =>
            place === index ? resizeAt(child, rest, sizes) : child,
        ),
    };
}

/** Give the parts of every split even shares. */
function evenOut(node: PaneNode): PaneNode {
    return node.kind === "leaf"
        ? node
        : {
              ...node,
              children: node.children.map((child) => evenOut(child)),
              sizes: evenSizes(node.children.length),
          };
}

/** List the boxes of a split's parts. */
function boxesOf(split: PaneSplit, box: PaneBox): readonly PaneBox[] {
    // line up one after the other
    let before = 0;

    return split.sizes.map((size) => {
        const at = before;
        before += size;

        return split.direction === "horizontal"
            ? { ...box, start: box.start + at * box.width, width: size * box.width }
            : { ...box, top: box.top + at * box.height, height: size * box.height };
    });
}

/** List where the panes of a node are. */
function placementsOf(node: PaneNode, box: PaneBox): readonly Omit<PanePlacement, "height">[] {
    // take the box, or share it out
    if (node.kind === "leaf") {
        return [{ pane: node, box }];
    }
    const boxes = boxesOf(node, box);

    return node.children.flatMap((child, index) => placementsOf(child, boxes[index] ?? box));
}

/** List the lines between the parts of a node's splits. */
function edgesOf(node: PaneNode, box: PaneBox, at: PanePath): readonly PaneEdge[] {
    // leave a pane
    if (node.kind === "leaf") {
        return [];
    }
    const { children, sizes, direction } = node;
    const boxes = boxesOf(node, box);
    const own = sizes.slice(0, -1).map((size, index): PaneEdge => {
        const [first, second] = [boxes[index] ?? box, boxes[index + 1] ?? box];
        const both =
            direction === "horizontal"
                ? { ...first, width: first.width + second.width }
                : { ...first, height: first.height + second.height };

        return {
            ...at,
            index,
            direction,
            box: both,
            share: size / (size + (sizes[index + 1] ?? 0)),
        };
    });
    const inner = children.flatMap((child, index) =>
        edgesOf(child, boxes[index] ?? box, { ...at, path: [...at.path, index] }),
    );

    return [...own, ...inner];
}

import { For, Show, type JSX } from "solid-js";

import {
    beginDrag,
    dragPayload,
    dragPointer,
    dropTarget,
    type DockEdge,
    type DockPlace,
} from "./drag";


/// The dock layout: nested splits with fractional sizes, view instances at the leaves.
export type DockNode =
    | { kind: "split"; direction: "row" | "column"; children: DockNode[]; sizes: number[] }
    | { kind: "leaf"; view: string };

/// Renderable content and label for one view instance.
/// Fields are accessors so reading them does not subscribe pane contents to layout changes.
export type ViewChrome = {
    isClosable: () => boolean;
    render: () => JSX.Element;
    title: () => string;
};

const minimumFraction = 0.08;

/// Move a view onto a target: swap positions or split against an edge.
export function moveView(root: DockNode, view: string, target: string, place: DockPlace): DockNode {
    if (view === target) {
        return root;
    }

    // header drops exchange the two leaves in place
    if (place === "swap") {
        return swapViews(root, view, target);
    }

    // detach the view, wrap the target leaf in a split holding both, flatten nesting
    const detached = removeView(root, view);
    if (detached == undefined) {
        return root;
    }

    return normalize(insertAt(detached, view, target, place));
}

/// Remove a view's leaf, returns undefined when the layout becomes empty.
export function removeView(node: DockNode, view: string): DockNode | undefined {
    if (node.kind === "leaf") {
        return node.view === view ? undefined : node;
    }

    // drop removed children and renormalize the remaining sizes
    const children: DockNode[] = [];
    const sizes: number[] = [];
    for (const [index, child] of node.children.entries()) {
        const kept = removeView(child, view);
        if (kept != undefined) {
            children.push(kept);
            sizes.push(node.sizes[index]);
        }
    }

    if (children.length === 0) {
        return undefined;
    }
    if (children.length === 1) {
        return children[0];
    }

    const total = sizes.reduce((sum, size) => sum + size, 0);

    return { ...node, children, sizes: sizes.map((size) => size / total) };
}

/// Append a view as a new column at the end of the layout.
export function appendView(root: DockNode, view: string): DockNode {
    const leaf: DockNode = { kind: "leaf", view };

    // join an existing root row, otherwise wrap into one
    if (root.kind === "split" && root.direction === "row") {
        const scale = 0.75;
        return {
            ...root,
            children: [...root.children, leaf],
            sizes: [...root.sizes.map((size) => size * scale), 0.25],
        };
    }

    return { kind: "split", direction: "row", children: [root, leaf], sizes: [0.75, 0.25] };
}

/// Resize a split's child boundary by a fraction delta, returns the new layout.
export function resizeSplit(
    root: DockNode,
    path: readonly number[],
    index: number,
    delta: number,
): DockNode {
    if (root.kind === "leaf") {
        return root;
    }

    // recurse to the addressed split
    if (path.length > 0) {
        const [head, ...rest] = path;
        const children = root.children.map((child, position) =>
            position === head ? resizeSplit(child, rest, index, delta) : child,
        );

        return { ...root, children };
    }

    // shift the boundary between index and index + 1 within bounds
    const sizes = [...root.sizes];
    const shift = Math.max(
        -(sizes[index] - minimumFraction),
        Math.min(delta, sizes[index + 1] - minimumFraction),
    );
    sizes[index] += shift;
    sizes[index + 1] -= shift;

    return { ...root, sizes };
}

/// All view instances present in a layout, used to validate persisted state.
export function dockViews(node: DockNode): string[] {
    if (node.kind === "leaf") {
        return [node.view];
    }

    return node.children.flatMap(dockViews);
}

function swapViews(node: DockNode, first: string, second: string): DockNode {
    if (node.kind === "leaf") {
        if (node.view === first) {
            return { kind: "leaf", view: second };
        }
        if (node.view === second) {
            return { kind: "leaf", view: first };
        }

        return node;
    }

    return { ...node, children: node.children.map((child) => swapViews(child, first, second)) };
}

/// Merge same-direction nested splits so sibling drops reorder instead of nesting.
function normalize(node: DockNode): DockNode {
    if (node.kind === "leaf") {
        return node;
    }

    // normalize children, then inline any child split sharing this direction
    const children: DockNode[] = [];
    const sizes: number[] = [];
    for (const [index, raw] of node.children.entries()) {
        const child = normalize(raw);
        if (child.kind === "split" && child.direction === node.direction) {
            for (const [inner, grandchild] of child.children.entries()) {
                children.push(grandchild);
                sizes.push(node.sizes[index] * child.sizes[inner]);
            }
        } else {
            children.push(child);
            sizes.push(node.sizes[index]);
        }
    }

    if (children.length === 1) {
        return children[0];
    }

    return { ...node, children, sizes };
}

function insertAt(node: DockNode, view: string, target: string, edge: DockEdge): DockNode {
    const direction = edge === "left" || edge === "right" ? "row" : "column";
    const isBefore = edge === "left" || edge === "top";

    if (node.kind === "leaf") {
        if (node.view !== target) {
            return node;
        }

        // wrap the target in a split holding the dropped view beside it
        const dropped: DockNode = { kind: "leaf", view };
        return {
            kind: "split",
            direction,
            children: isBefore ? [dropped, node] : [node, dropped],
            sizes: [0.5, 0.5],
        };
    }

    return { ...node, children: node.children.map((child) => insertAt(child, view, target, edge)) };
}

type DockViewProps = {
    chrome: (view: string) => ViewChrome;
    node: DockNode;
    onClose: (view: string) => void;
    onMove: (view: string, target: string, place: DockPlace) => void;
    onResize: (path: readonly number[], index: number, delta: number) => void;
    path?: readonly number[];
};

export function DockView(props: DockViewProps) {
    const path = () => props.path ?? [];

    return (
        <Show
            fallback={<DockLeaf {...props} view={(props.node as { view: string }).view} />}
            when={props.node.kind === "split" && props.node}
        >
            {(split) => (
                <div
                    class="grid h-full min-h-0 min-w-0"
                    style={gridStyle(split().direction, split().sizes)}
                >
                    <For each={split().children}>
                        {(child, index) => (
                            <>
                                <Show when={index() > 0}>
                                    <DockHandle
                                        direction={split().direction}
                                        onResize={(delta) =>
                                            props.onResize(path(), index() - 1, delta)
                                        }
                                    />
                                </Show>
                                <DockView
                                    chrome={props.chrome}
                                    node={child}
                                    onClose={props.onClose}
                                    onMove={props.onMove}
                                    onResize={props.onResize}
                                    path={[...path(), index()]}
                                />
                            </>
                        )}
                    </For>
                </div>
            )}
        </Show>
    );
}

type DockHandleProps = {
    direction: "row" | "column";
    onResize: (delta: number) => void;
};

function DockHandle(props: DockHandleProps) {
    // convert pointer movement into fraction deltas of the parent split
    const startResize = (event: PointerEvent) => {
        const container = (event.currentTarget as HTMLElement).parentElement;
        if (container == undefined) {
            return;
        }

        const bounds = container.getBoundingClientRect();
        const extent = props.direction === "row" ? bounds.width : bounds.height;
        let last = props.direction === "row" ? event.clientX : event.clientY;

        dragPointer(event, (clientX, clientY) => {
            const position = props.direction === "row" ? clientX : clientY;
            props.onResize((position - last) / extent);
            last = position;
        });
    };

    return (
        <button
            aria-label="resize panes"
            class="h-full min-h-0 w-full min-w-0 touch-none appearance-none border-0 bg-editor-line p-0 transition-colors hover:bg-destack-accent"
            classList={{
                "cursor-col-resize": props.direction === "row",
                "cursor-row-resize": props.direction === "column",
            }}
            onPointerDown={startResize}
            type="button"
        />
    );
}

type DockLeafProps = DockViewProps & {
    view: string;
};

const headerDragHeight = 32;

function DockLeaf(props: DockLeafProps) {
    const chrome = props.chrome(props.view);
    const overPlace = () => {
        const target = dropTarget();
        return target?.kind === "view" && target.id === props.view ? target.place : undefined;
    };

    const grab = (event: PointerEvent) => {
        beginDrag(event, { kind: "view", id: props.view, label: chrome.title() }, (target) => {
            if (target?.kind === "view") {
                props.onMove(props.view, target.id, target.place);
            }
        });
    };

    // the whole header strip drags the pane; tabs and editors handle their own pointers
    const grabHeader = (event: PointerEvent) => {
        const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect();
        const hasOwnDrag =
            event.target instanceof Element &&
            event.target.closest("[data-drop-tab], input, .cm-editor") != undefined;

        if (!hasOwnDrag && event.clientY - bounds.top <= headerDragHeight) {
            grab(event);
        }
    };

    return (
        <section
            class="relative h-full min-h-0 min-w-0 overflow-hidden"
            data-drop-view={props.view}
            onPointerDown={grabHeader}
        >
            {/* Grip and close, over the pane's header row */}
            <div class="absolute top-1 right-1 z-30 flex items-center">
                <span
                    aria-hidden="true"
                    class="grid h-6 w-5 cursor-grab touch-none place-items-center text-editor-muted/50 hover:text-editor-text"
                >
                    ⠿
                </span>
                <Show when={chrome.isClosable()}>
                    <button
                        aria-label={`close ${chrome.title()}`}
                        class="grid h-6 w-5 place-items-center text-editor-muted/50 hover:text-editor-text"
                        onClick={() => props.onClose(props.view)}
                        type="button"
                    >
                        ×
                    </button>
                </Show>
            </div>

            {chrome.render()}

            {/* Drop highlight while another view is dragged over this one */}
            <Show when={dragPayload()?.kind === "view" && overPlace()}>
                {(place) => (
                    <div
                        class="pointer-events-none absolute z-20 bg-destack-accent/25"
                        classList={{
                            "inset-0": place() === "swap",
                            "inset-y-0 left-0 w-1/2": place() === "left",
                            "inset-y-0 right-0 w-1/2": place() === "right",
                            "inset-x-0 top-0 h-1/2": place() === "top",
                            "inset-x-0 bottom-0 h-1/2": place() === "bottom",
                        }}
                    />
                )}
            </Show>
        </section>
    );
}

function gridStyle(direction: "row" | "column", sizes: readonly number[]) {
    // interleave fractional tracks with fixed handle tracks
    const tracks = sizes.map((size) => `minmax(0, ${size}fr)`).join(" 3px ");

    return direction === "row"
        ? `grid-template-columns: ${tracks}`
        : `grid-template-rows: ${tracks}`;
}

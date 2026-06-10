import { Show, createSignal } from "solid-js";

/// Edge of a view a drop lands on.
export type DockEdge = "left" | "right" | "top" | "bottom";

/// Where a dropped view lands: split against an edge, or swap via the header.
export type DockPlace = DockEdge | "swap";

/// What a drag is carrying.
export type DragPayload =
    | { kind: "file"; path: string; label: string }
    | { kind: "tab"; path: string; label: string }
    | { kind: "view"; id: string; label: string };

/// What the pointer is currently over, resolved by hit-testing.
export type DropTarget =
    | { kind: "directory"; path: string }
    | { kind: "tab"; path: string }
    | { kind: "view"; id: string; place: DockPlace };

const headerSwapHeight = 32;

const dragThreshold = 4;

/// One drag can exist at a time, so the controller is a singleton.
type DragController = {
    drag: () => { payload: DragPayload; x: number; y: number } | undefined;
    setDrag: (state: { payload: DragPayload; x: number; y: number } | undefined) => void;
    target: () => DropTarget | undefined;
    setTarget: (state: DropTarget | undefined) => void;
};

// stash the singleton on globalThis so dev hot reloads keep one shared instance
function controller(): DragController {
    const host = globalThis as { __destackDrag?: DragController };
    if (host.__destackDrag == undefined) {
        const [drag, setDrag] = createSignal<
            { payload: DragPayload; x: number; y: number } | undefined
        >(undefined);
        const [target, setTarget] = createSignal<DropTarget | undefined>(undefined);
        host.__destackDrag = { drag, setDrag, target, setTarget };
    }

    return host.__destackDrag;
}

const { drag, setDrag, target, setTarget } = controller();

/// The payload currently being dragged, if any.
export function dragPayload() {
    return drag()?.payload;
}

/// The drop target currently under the pointer, if any.
export function dropTarget() {
    return target();
}

/// Track a pointer drag from this pointerdown; clicks below the threshold stay clicks.
export function beginDrag(
    event: PointerEvent,
    payload: DragPayload,
    onDrop: (target: DropTarget | undefined) => void,
) {
    const origin = { x: event.clientX, y: event.clientY };
    let isActive = false;

    const move = (moveEvent: PointerEvent) => {
        // arm only after real movement so plain clicks pass through
        if (!isActive) {
            const distance = Math.hypot(moveEvent.clientX - origin.x, moveEvent.clientY - origin.y);
            if (distance < dragThreshold) {
                return;
            }
            isActive = true;

            // keep the page from selecting text under the drag
            document.body.style.userSelect = "none";
            document.body.style.cursor = "grabbing";
        }

        setDrag({ payload, x: moveEvent.clientX, y: moveEvent.clientY });
        setTarget(resolveTarget(moveEvent.clientX, moveEvent.clientY, payload));
    };
    const stop = () => {
        window.removeEventListener("pointermove", move);
        window.removeEventListener("pointerup", stop);
        window.removeEventListener("pointercancel", stop);
        document.body.style.userSelect = "";
        document.body.style.cursor = "";

        const resolved = target();
        setDrag(undefined);
        setTarget(undefined);

        // swallow the click that follows a completed drag
        if (isActive) {
            window.addEventListener("click", suppressClick, { capture: true, once: true });
            setTimeout(() => window.removeEventListener("click", suppressClick, true), 0);
            onDrop(resolved);
        }
    };

    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop);
    window.addEventListener("pointercancel", stop);
}

/// Floating label following the pointer during a drag, rendered once per shell.
export function DragGhost() {
    return (
        <Show when={drag()}>
            {(state) => (
                <div
                    class="pointer-events-none fixed z-50 border border-destack-frame bg-editor-window px-2 py-0.5 text-xs text-editor-text shadow-lg"
                    style={`left: ${state().x + 10}px; top: ${state().y + 8}px`}
                >
                    {state().payload.label}
                </div>
            )}
        </Show>
    );
}

function resolveTarget(x: number, y: number, payload: DragPayload): DropTarget | undefined {
    const element = document.elementFromPoint(x, y);
    if (element == undefined) {
        return undefined;
    }

    // files land in directories
    if (payload.kind === "file") {
        const directory = element.closest("[data-drop-directory]");
        const path = directory?.getAttribute("data-drop-directory");
        return path == undefined ? undefined : { kind: "directory", path };
    }

    // tabs reorder among tabs
    if (payload.kind === "tab") {
        const tab = element.closest("[data-drop-tab]");
        const path = tab?.getAttribute("data-drop-tab");
        return path == undefined ? undefined : { kind: "tab", path };
    }

    // views swap via the target's header, otherwise split on the nearest edge
    const view = element.closest("[data-drop-view]");
    const id = view?.getAttribute("data-drop-view");
    if (view == undefined || id == undefined || id === payload.id) {
        return undefined;
    }

    const bounds = view.getBoundingClientRect();
    if (y - bounds.top <= headerSwapHeight) {
        return { kind: "view", id, place: "swap" };
    }

    return { kind: "view", id, place: nearestEdge(bounds, x, y) };
}

function nearestEdge(bounds: DOMRect, x: number, y: number): DockEdge {
    const relativeX = (x - bounds.left) / bounds.width;
    const relativeY = (y - bounds.top) / bounds.height;

    const distances: [DockEdge, number][] = [
        ["left", relativeX],
        ["right", 1 - relativeX],
        ["top", relativeY],
        ["bottom", 1 - relativeY],
    ];
    distances.sort((a, b) => a[1] - b[1]);

    return distances[0][0];
}

function suppressClick(event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
}

/// Track a pointer drag that only needs positions, like a splitter.
export function dragPointer(
    event: PointerEvent,
    onMove: (clientX: number, clientY: number) => void,
) {
    event.preventDefault();

    const target = event.currentTarget as HTMLElement;
    target.setPointerCapture?.(event.pointerId);

    const move = (moveEvent: PointerEvent) => {
        onMove(moveEvent.clientX, moveEvent.clientY);
    };
    const stop = () => {
        target.releasePointerCapture?.(event.pointerId);
        window.removeEventListener("pointermove", move);
        window.removeEventListener("pointerup", stop);
        window.removeEventListener("pointercancel", stop);
    };

    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop);
    window.addEventListener("pointercancel", stop);
}

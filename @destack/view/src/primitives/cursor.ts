import { isServer } from "@solidjs/web";
import { type Accessor, createEffect, createSignal } from "solid-js";
import { access, type FalsyValue, type MaybeAccessor, TRANSPARENT } from "./utils.ts";

/** A value of the CSS `cursor` property. */
export type CursorProperty =
    | "alias"
    | "all-scroll"
    | "auto"
    | "cell"
    | "col-resize"
    | "context-menu"
    | "copy"
    | "crosshair"
    | "default"
    | "e-resize"
    | "ew-resize"
    | "grab"
    | "grabbing"
    | "help"
    | "move"
    | "n-resize"
    | "ne-resize"
    | "nesw-resize"
    | "no-drop"
    | "none"
    | "not-allowed"
    | "ns-resize"
    | "nw-resize"
    | "nwse-resize"
    | "pointer"
    | "progress"
    | "row-resize"
    | "s-resize"
    | "se-resize"
    | "sw-resize"
    | "text"
    | "vertical-text"
    | "w-resize"
    | "wait"
    | "zoom-in"
    | "zoom-out"
    | (string & {});

/** Show a cursor over the body now, returning how to restore the previous one. */
export function makeBodyCursor(value: CursorProperty): () => void {
    // set nothing on the server
    if (isServer) {
        return () => {};
    }

    return makeElementCursor(document.body, value);
}

/** Show a cursor over an element now, returning how to restore the previous one. */
export function makeElementCursor(target: HTMLElement, value: CursorProperty): () => void {
    // set nothing on the server
    if (isServer) {
        return () => {};
    }

    // remember the previous cursor and its priority to put back
    const previous = target.style.getPropertyValue("cursor");
    const priority = target.style.getPropertyPriority("cursor");
    target.style.setProperty("cursor", value, "important");

    return () => {
        if (previous === "") {
            target.style.removeProperty("cursor");
        } else {
            target.style.setProperty("cursor", previous, priority);
        }
    };
}

/** Show a cursor over an element, following both, and none while the element is absent. */
export function createElementCursor(
    target: Accessor<HTMLElement | FalsyValue> | HTMLElement,
    value: MaybeAccessor<CursorProperty>,
): void {
    // show nothing on the server
    if (isServer) {
        return;
    }
    createEffect(
        () => ({ element: access(target), shown: access(value) }),
        ({ element, shown }) =>
            element instanceof HTMLElement ? makeElementCursor(element, shown) : undefined,
        TRANSPARENT,
    );
}

/** Show a cursor over the body while an accessor gives one. */
export function createBodyCursor(value: Accessor<CursorProperty | FalsyValue>): void {
    // show nothing on the server
    if (isServer) {
        return;
    }
    createEffect(
        value,
        (shown) => (typeof shown === "string" && shown !== "" ? makeBodyCursor(shown) : undefined),
        TRANSPARENT,
    );
}

/** Show a grab cursor over an element, and a grabbing cursor over the whole body while it is dragged. */
export function createDragCursor(
    target: Accessor<HTMLElement | FalsyValue> | HTMLElement,
    options?: { readonly grab?: CursorProperty; readonly grabbing?: CursorProperty },
): void {
    // show nothing on the server
    if (isServer) {
        return;
    }

    // grab over the element, and grabbing over the body while dragging, which the element must not override
    const grab = options?.grab ?? "grab";
    const grabbing = options?.grabbing ?? "grabbing";
    const [isDragging, setIsDragging] = createSignal(false, { ownedWrite: true });
    createBodyCursor(() => isDragging() && grabbing);
    createElementCursor(() => (isDragging() ? false : access(target)), grab);

    // drag from a press on the element until the pointer lifts or is cancelled anywhere
    createEffect(
        () => access(target),
        (element) => {
            // stop dragging while there is no element
            if (!(element instanceof HTMLElement)) {
                setIsDragging(false);

                return undefined;
            }
            const press = (): void => {
                setIsDragging(true);
            };
            const lift = (): void => {
                setIsDragging(false);
            };
            element.addEventListener("pointerdown", press);
            document.addEventListener("pointerup", lift);
            document.addEventListener("pointercancel", lift);

            return () => {
                element.removeEventListener("pointerdown", press);
                document.removeEventListener("pointerup", lift);
                document.removeEventListener("pointercancel", lift);
            };
        },
        TRANSPARENT,
    );
}

/** Show a cursor over the element a ref receives, following a reactive cursor, until the calling owner is cleaned up. */
export function cursor(value: MaybeAccessor<CursorProperty>): (element: HTMLElement) => void {
    // follow the cursor in the calling owner, and only take the element in the ref
    const [target, setTarget] = createSignal<HTMLElement | undefined>(undefined, {
        ownedWrite: true,
    });
    createElementCursor(target, value);

    return (element) => {
        setTarget(() => element);
    };
}

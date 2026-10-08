import { isServer } from "@solidjs/web";
import { type Accessor, createEffect, onSettled } from "solid-js";
import { createEventListener, makeEventListener } from "./event-listener.ts";
import { access, createHydratableSignal, type FalsyValue, type MaybeAccessor } from "./utils.ts";

/** Call back as an element gains and loses the focus, until cleanup or the returned function. */
export function makeFocusListener(
    target: Element,
    callback: (isActive: boolean) => void,
    useCapture = true,
): () => void {
    // listen nowhere on the server
    if (isServer) {
        return () => {};
    }
    const stopBlur = makeEventListener(target, "blur", () => callback(false), useCapture);
    const stopFocus = makeEventListener(target, "focus", () => callback(true), useCapture);

    return () => {
        stopBlur();
        stopFocus();
    };
}

/** Follow whether an element holds the focus. */
export function createFocusSignal(target: MaybeAccessor<Element>): Accessor<boolean> {
    // follow nothing on the server
    if (isServer) {
        return () => false;
    }

    // start from whether the element holds the focus, then follow its focus and blur
    const [isActive, setIsActive] = createHydratableSignal(
        false,
        () => document.activeElement === access(target),
    );
    createEventListener(target, "blur", () => setIsActive(false), true);
    createEventListener(target, "focus", () => setIsActive(true), true);

    return isActive;
}

/** Focus the element the ref receives once rendered, when it has the `autofocus` attribute. */
export function autofocus(): (element: HTMLElement) => void {
    // focus the element after the task that renders it, while it asks for focus
    let target: HTMLElement | undefined;
    onSettled(() => {
        if (target?.hasAttribute("autofocus") !== true) {
            return undefined;
        }
        const element = target;
        const handle = setTimeout(() => element.focus());

        return () => clearTimeout(handle);
    });

    return (element) => {
        target = element;
    };
}

/** Focus each element an accessor gives once rendered. */
export function createAutofocus(target: Accessor<HTMLElement | FalsyValue>): void {
    createEffect(target, (element) => {
        if (!(element instanceof HTMLElement)) {
            return undefined;
        }
        const handle = setTimeout(() => element.focus());

        return () => clearTimeout(handle);
    });
}

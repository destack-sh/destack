/** What interacts with an element. */
export type PointerType = "mouse" | "pen" | "touch" | "keyboard" | "virtual";

/** One step of an interaction with an element. */
export interface InteractionEvent<Type extends string = string> {
    /** The step. */
    readonly type: Type;
    /** What interacts. */
    readonly pointerType: PointerType;
    /** The element. */
    readonly target: HTMLElement;
    /** The horizontal position within the element. */
    readonly x: number;
    /** The vertical position within the element. */
    readonly y: number;
    /** Whether shift was held. */
    readonly shiftKey: boolean;
    /** Whether command was held. */
    readonly metaKey: boolean;
    /** Whether control was held. */
    readonly ctrlKey: boolean;
    /** Whether option or alt was held. */
    readonly altKey: boolean;
    /** Whether the browser took the interaction over. */
    readonly isCancelled: boolean;
}

/** Read the type of a pointer, anything but a finger or a pen as a mouse. */
export function pointerTypeOf(event: PointerEvent): PointerType {
    return event.pointerType === "touch" || event.pointerType === "pen"
        ? event.pointerType
        : "mouse";
}

/** Read an interaction's step on an element from the DOM event at hand. */
export function interactionOf<Type extends string>(
    type: Type,
    pointerType: PointerType,
    target: HTMLElement,
    event: Event,
    isCancelled: boolean,
): InteractionEvent<Type> {
    // place a pointer within the element, and a key or a click without a pointer at its center
    const bounds = target.getBoundingClientRect();
    const isPointed =
        pointerType !== "keyboard" && pointerType !== "virtual" && event instanceof MouseEvent;
    const x = isPointed ? event.clientX - bounds.left : bounds.width / 2;
    const y = isPointed ? event.clientY - bounds.top : bounds.height / 2;

    // read the held keys, none on an event without them
    const isKeyed = event instanceof MouseEvent || event instanceof KeyboardEvent;

    return {
        type,
        pointerType,
        target,
        x,
        y,
        shiftKey: isKeyed && event.shiftKey,
        metaKey: isKeyed && event.metaKey,
        ctrlKey: isKeyed && event.ctrlKey,
        altKey: isKeyed && event.altKey,
        isCancelled,
    };
}

/** Read the center of an element in the viewport. */
export function centerOf(element: HTMLElement): { readonly x: number; readonly y: number } {
    const bounds = element.getBoundingClientRect();

    return { x: bounds.left + bounds.width / 2, y: bounds.top + bounds.height / 2 };
}

/** Report whether a screen reader sent a pointer event, which has no size, or TalkBack's mouse. */
export function isVirtualPointer(event: PointerEvent): boolean {
    // read the platform and the marks of each screen reader's pointer
    const isAndroid = typeof navigator !== "undefined" && navigator.userAgent.includes("Android");
    const isSized = event.width !== 0 || event.height !== 0;
    const isTalkBack =
        event.width === 1 &&
        event.height === 1 &&
        event.pressure === 0 &&
        event.detail === 0 &&
        event.pointerType === "mouse";

    return isAndroid ? isTalkBack : !isSized;
}

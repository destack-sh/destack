import { isServer } from "@solidjs/web";
import { type Accessor, createSignal, getOwner } from "solid-js";
import { type Many, type MaybeAccessor } from "./utils.ts";
import { createEventListener } from "./event-listener.ts";
import { createSubRoot } from "./rootless.ts";

/** The kind of device a pointer is. */
export type PointerType = "mouse" | "touch" | "pen";

/** A target pointer listeners attach to. */
export type PointerTarget = Window | Document | HTMLElement;

/** The pointer events by the rest of their name after `pointer`. */
export type PointerEventNames =
    | "over"
    | "enter"
    | "down"
    | "move"
    | "up"
    | "cancel"
    | "out"
    | "leave"
    | "gotCapture"
    | "lostCapture";

/** An `on` handler name in either casing, such as `onmove` or `onMove`. */
export type OnEventName<Name extends string> = `on${Lowercase<Name>}` | `on${Capitalize<Name>}`;

/** A value under each `on` handler name of some events. */
export type OnEventRecord<Name extends string, Value> = Record<OnEventName<Name>, Value>;

/** Handle a pointer event. */
export type Handler = (event: PointerEvent) => void;

/** A pointer's position and properties. */
export type PointerState = {
    /** The pressure, from zero to one. */
    readonly pressure: number;
    /** The pointer's identifier. */
    readonly pointerId: number;
    /** The tilt along the x axis, in degrees. */
    readonly tiltX: number;
    /** The tilt along the y axis, in degrees. */
    readonly tiltY: number;
    /** The contact width. */
    readonly width: number;
    /** The contact height. */
    readonly height: number;
    /** The rotation around the pointer's axis, in degrees. */
    readonly twist: number;
    /** The kind of device, null before any pointer. */
    readonly pointerType: PointerType | null;
    /** The horizontal position in the viewport. */
    readonly x: number;
    /** The vertical position in the viewport. */
    readonly y: number;
};

/** A pointer's state, and whether it is over the target. */
export type PointerStateWithActive = PointerState & {
    /** Whether the pointer is over the target. */
    readonly isActive: boolean;
};

/** A pointer's state in a list of pointers, and whether it is down. */
export type PointerListItem = PointerState & {
    /** Whether the pointer is down. */
    readonly isDown: boolean;
};

/** Handle a pointer's position over an element. */
export type PointerPositionDirectiveHandler = (
    state: PointerStateWithActive,
    element: Element,
) => void;

/** The handler of `pointerPosition`, alone or with the pointer types it follows. */
export type PointerPositionDirectiveProperties =
    | PointerPositionDirectiveHandler
    | { readonly pointerTypes?: PointerType[]; readonly handler: PointerPositionDirectiveHandler };

/** Handle whether an element is hovered. */
export type PointerHoverDirectiveHandler = (isHovering: boolean, element: Element) => void;

/** The handler of `pointerHover`, alone or with the pointer types it follows. */
export type PointerHoverDirectiveProperties =
    | PointerHoverDirectiveHandler
    | { readonly pointerTypes?: PointerType[]; readonly handler: PointerHoverDirectiveHandler };

/** The handlers a followed pointer may register while it enters, by name in either casing. */
export type PerPointerHandlers = Readonly<
    OnEventRecord<"down" | "move" | "up" | "leave" | "cancel", (handler: Handler) => void>
>;

/** The state before any pointer. */
const DEFAULT_STATE: PointerStateWithActive = {
    x: 0,
    y: 0,
    pointerId: 0,
    pressure: 0,
    tiltX: 0,
    tiltY: 0,
    width: 0,
    height: 0,
    twist: 0,
    pointerType: null,
    isActive: false,
};

/** Move a viewport position into an element's coordinates, telling whether it falls inside. */
export function getPositionToElement<Position extends { readonly x: number; readonly y: number }>(
    position: Position,
    element: Element,
): Position & { isInside: boolean } {
    // measure the position from the element's corner
    const { top, left, width, height } = element.getBoundingClientRect();
    const x = position.x - left;
    const y = position.y - top;

    return { ...position, x, y, isInside: x >= 0 && y >= 0 && x <= width && y <= height };
}

/** Listen to pointer events of the chosen pointer kinds on a target, `document.body` by default, until cleanup. */
export function createPointerListeners(
    configuration: Partial<OnEventRecord<PointerEventNames, Handler>> & {
        readonly target?: MaybeAccessor<PointerTarget | undefined>;
        readonly pointerTypes?: PointerType[];
        readonly passive?: boolean;
    },
): void {
    // listen to each handler's event, on both sides to keep hydration keys aligned
    const target = configuration.target ?? (isServer ? undefined : document.body);
    for (const [key, handler] of Object.entries(configuration)) {
        if (key.startsWith("on") && typeof handler === "function") {
            const name = key.slice(2).toLowerCase();
            const type =
                name === "gotcapture" || name === "lostcapture"
                    ? `${name.slice(0, -7)}pointercapture`
                    : `pointer${name}`;
            listen(
                target,
                type,
                (event) => {
                    if (isOfTypes(event, configuration.pointerTypes)) {
                        handler(event);
                    }
                },
                configuration.passive ?? true,
            );
        }
    }
}

/** Follow each pointer from when it enters or goes down until it leaves or goes up, with handlers it registers right away. */
export function createPerPointerListeners(
    configuration: {
        readonly target?: MaybeAccessor<PointerTarget>;
        readonly pointerTypes?: PointerType[];
        readonly passive?: boolean;
    } & Partial<
        OnEventRecord<"enter", (event: PointerEvent, handlers: PerPointerHandlers) => void> &
            OnEventRecord<
                "down",
                (
                    event: PointerEvent,
                    onMove: (handler: Handler) => void,
                    onUp: (handler: Handler) => void,
                ) => void
            >
    >,
): void {
    // listen for pointers of the chosen kinds, and of one pointer once followed
    const target = configuration.target ?? (isServer ? undefined : document.body);
    const passive = configuration.passive ?? true;
    const owner = getOwner();
    const add = (type: Many<string>, handler: Handler, pointerId?: number): void =>
        listen(
            target,
            type,
            (event) => {
                if (
                    isOfTypes(event, configuration.pointerTypes) &&
                    (pointerId === undefined || event.pointerId === pointerId)
                ) {
                    handler(event);
                }
            },
            passive,
        );

    // follow a pointer from when it enters, until it leaves
    const onEnter = configuration.onEnter ?? configuration.onenter;
    if (onEnter !== undefined) {
        add("pointerenter", (entered) => {
            createSubRoot((dispose) => {
                // follow the pointer until it leaves
                let isInitial = true;
                let onLeave: Handler | undefined;
                add(
                    "pointerleave",
                    (event) => {
                        onLeave?.(event);
                        dispose();
                    },
                    entered.pointerId,
                );
                const register =
                    (type: string) =>
                    (handler: Handler): void => {
                        if (!isInitial) {
                            throw new TypeError(
                                "pointer listeners are added while the pointer's first event runs",
                            );
                        }
                        if (type === "pointerleave") {
                            onLeave = handler;
                        } else {
                            add(type, handler, entered.pointerId);
                        }
                    };
                onEnter(entered, {
                    ondown: register("pointerdown"),
                    onDown: register("pointerdown"),
                    onmove: register("pointermove"),
                    onMove: register("pointermove"),
                    onup: register("pointerup"),
                    onUp: register("pointerup"),
                    onleave: register("pointerleave"),
                    onLeave: register("pointerleave"),
                    oncancel: register("pointercancel"),
                    onCancel: register("pointercancel"),
                });
                isInitial = false;
            }, owner);
        });
    }

    // follow a pointer from when it goes down, until it goes up or is cancelled
    const onDown = configuration.onDown ?? configuration.ondown;
    if (onDown !== undefined) {
        add("pointerdown", (pressed) => {
            createSubRoot((dispose) => {
                // follow the pointer until it goes up
                let isInitial = true;
                let onUp: Handler | undefined;
                add(
                    ["pointerup", "pointercancel"],
                    (event) => {
                        onUp?.(event);
                        dispose();
                    },
                    pressed.pointerId,
                );
                const guard =
                    (register: (handler: Handler) => void) =>
                    (handler: Handler): void => {
                        if (!isInitial) {
                            throw new TypeError(
                                "pointer listeners are added while the pointer's first event runs",
                            );
                        }
                        register(handler);
                    };
                onDown(
                    pressed,
                    guard((handler) => add("pointermove", handler, pressed.pointerId)),
                    guard((handler) => {
                        onUp = handler;
                    }),
                );
                isInitial = false;
            }, owner);
        });
    }
}

/** Follow the position of the first pointer over a target. */
export function createPointerPosition(
    configuration: {
        readonly target?: MaybeAccessor<PointerTarget>;
        readonly pointerTypes?: PointerType[];
        readonly value?: PointerStateWithActive;
    } = {},
): Accessor<PointerStateWithActive> {
    // hold the state of the pointer followed, starting from the given value
    const [state, setState] = createSignal<PointerStateWithActive>(
        configuration.value ?? DEFAULT_STATE,
        {
            ownedWrite: true,
        },
    );
    let pointer: number | null = null;
    const update = (event: PointerEvent, isActive = true): void => {
        setState({ ...toState(event), isActive });
    };

    // follow the first pointer that enters until it leaves
    createPointerListeners({
        ...(configuration.target === undefined ? {} : { target: configuration.target }),
        ...(configuration.pointerTypes === undefined
            ? {}
            : { pointerTypes: configuration.pointerTypes }),
        onEnter: (event) => {
            if (pointer === null) {
                pointer = event.pointerId;
                update(event);
            }
        },
        onMove: (event) => {
            if (event.pointerId === pointer) {
                update(event);
            }
        },
        onLeave: (event) => {
            if (event.pointerId === pointer) {
                pointer = null;
                update(event, false);
            }
        },
    });

    return isServer ? () => DEFAULT_STATE : state;
}

/** Follow every pointer on a target, each in a signal of its own. */
export function createPointerList(
    configuration: {
        readonly target?: MaybeAccessor<PointerTarget>;
        readonly pointerTypes?: PointerType[];
    } = {},
): Accessor<Accessor<PointerListItem>[]> {
    // hold a signal per pointer from when it enters until it leaves
    const [pointers, setPointers] = createSignal<Accessor<PointerListItem>[]>([], {
        ownedWrite: true,
    });
    createPerPointerListeners({
        ...configuration,
        onEnter(entered, { onMove, onDown, onUp, onLeave }) {
            // hold the pointer's state
            const [pointer, setPointer] = createSignal<PointerListItem>(
                { ...toState(entered), isDown: false },
                { ownedWrite: true },
            );
            setPointers((current) => [...current, pointer]);
            onMove((event) =>
                setPointer((previous) => ({ ...toState(event), isDown: previous.isDown })),
            );
            onDown((event) => setPointer({ ...toState(event), isDown: true }));
            onUp((event) => setPointer({ ...toState(event), isDown: false }));
            onLeave(() => setPointers((current) => current.filter((item) => item !== pointer)));
        },
    });

    return isServer ? () => [] : pointers;
}

/** Report the position of the first pointer over the element a ref receives. */
export function pointerPosition(
    properties: PointerPositionDirectiveProperties,
): (element: Element) => void {
    // follow the first pointer over the element once the ref receives it
    const [target, setTarget] = createSignal<HTMLElement | undefined>(undefined, {
        ownedWrite: true,
    });
    const { handler, pointerTypes } =
        typeof properties === "function"
            ? { handler: properties, pointerTypes: undefined }
            : properties;
    let pointer: number | null = null;
    const report = (event: PointerEvent, isActive = true): void => {
        const element = target();
        if (element !== undefined) {
            handler({ ...toState(event), isActive }, element);
        }
    };
    createPointerListeners({
        target,
        ...(pointerTypes === undefined ? {} : { pointerTypes }),
        onEnter: (event) => {
            if (pointer === null) {
                pointer = event.pointerId;
                report(event);
            }
        },
        onMove: (event) => {
            if (event.pointerId === pointer) {
                report(event);
            }
        },
        onLeave: (event) => {
            if (event.pointerId === pointer) {
                pointer = null;
                report(event, false);
            }
        },
    });

    return (element) => {
        if (element instanceof HTMLElement) {
            setTarget(element);
        }
    };
}

/** Report whether at least one pointer hovers the element a ref receives. */
export function pointerHover(
    properties: PointerHoverDirectiveProperties,
): (element: Element) => void {
    // count the pointers over the element once the ref receives it
    const [target, setTarget] = createSignal<HTMLElement | undefined>(undefined, {
        ownedWrite: true,
    });
    const { handler, pointerTypes } =
        typeof properties === "function"
            ? { handler: properties, pointerTypes: undefined }
            : properties;
    const pointers = new Set<number>();
    createPointerListeners({
        target,
        ...(pointerTypes === undefined ? {} : { pointerTypes }),
        onEnter: (event) => {
            pointers.add(event.pointerId);
            const element = target();
            if (element !== undefined) {
                handler(true, element);
            }
        },
        onLeave: (event) => {
            pointers.delete(event.pointerId);
            const element = target();
            if (pointers.size === 0 && element !== undefined) {
                handler(false, element);
            }
        },
    });

    return (element) => {
        if (element instanceof HTMLElement) {
            setTarget(element);
        }
    };
}

/** Listen to pointer events of a type on a target. */
function listen(
    target: MaybeAccessor<PointerTarget | undefined>,
    type: Many<string>,
    handler: Handler,
    passive: boolean,
): void {
    createEventListener<{ [Name: string]: PointerEvent }>(target, type, handler, { passive });
}

/** Check whether a pointer event is of one of the chosen kinds, any kind when none is chosen. */
function isOfTypes(event: PointerEvent, pointerTypes: readonly PointerType[] | undefined): boolean {
    return pointerTypes === undefined || pointerTypes.some((type) => type === event.pointerType);
}

/** Read a pointer event's state. */
function toState(event: PointerEvent): PointerState {
    return {
        x: event.clientX,
        y: event.clientY,
        pointerId: event.pointerId,
        pressure: event.pressure,
        tiltX: event.tiltX,
        tiltY: event.tiltY,
        width: event.width,
        height: event.height,
        twist: event.twist,
        pointerType: pointerTypeOf(event.pointerType),
    };
}

/** Read a reported pointer kind, null for one the platform does not name. */
function pointerTypeOf(type: string): PointerType | null {
    return type === "mouse" || type === "touch" || type === "pen" ? type : null;
}

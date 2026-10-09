import { isServer } from "@solidjs/web";
import {
    type Accessor,
    type Component,
    createEffect,
    createRenderEffect,
    createSignal,
    getOwner,
    onCleanup,
} from "solid-js";
import { access, asArray, type Many, type MaybeAccessor, TRANSPARENT } from "./utils.ts";

/** The options of a listener, as `addEventListener` takes them. */
export type EventListenerOptions = boolean | AddEventListenerOptions;

/** The targets whose events the DOM types by an event map. */
export type TargetWithEventMap =
    | Window
    | Document
    | XMLDocument
    | HTMLBodyElement
    | HTMLMediaElement
    | HTMLVideoElement
    | HTMLElement
    | SVGSVGElement
    | SVGElement
    | MathMLElement
    | Element
    | AbortSignal
    | AbstractWorker
    | Animation
    | BroadcastChannel
    | CSSAnimation
    | CSSTransition
    | FileReader
    | IDBDatabase
    | IDBOpenDBRequest
    | IDBRequest
    | IDBTransaction
    | MediaDevices
    | MediaKeySession
    | MediaQueryList
    | MediaRecorder
    | MediaSource
    | MediaStream
    | MediaStreamTrack
    | MessagePort
    | Notification
    | PaymentRequest
    | Performance
    | PermissionStatus
    | PictureInPictureWindow
    | RemotePlayback
    | ScreenOrientation
    | ServiceWorker
    | ServiceWorkerContainer
    | ServiceWorkerRegistration
    | ShadowRoot
    | SharedWorker
    | SourceBuffer
    | SourceBufferList
    | SpeechSynthesis
    | SpeechSynthesisUtterance
    | VisualViewport
    | WebSocket
    | Worker
    | XMLHttpRequest
    | XMLHttpRequestEventTarget
    | XMLHttpRequestUpload
    | EventSource;

/** The event map the DOM types a target's events by. */
export type EventMapOf<Target> = Target extends Window
    ? WindowEventMap
    : Target extends Document | XMLDocument
      ? DocumentEventMap
      : Target extends HTMLBodyElement
        ? HTMLBodyElementEventMap
        : Target extends HTMLMediaElement
          ? HTMLMediaElementEventMap
          : Target extends HTMLElement
            ? HTMLElementEventMap
            : Target extends SVGSVGElement
              ? SVGSVGElementEventMap
              : Target extends SVGElement
                ? SVGElementEventMap
                : Target extends MathMLElement
                  ? MathMLElementEventMap
                  : Target extends Element
                    ? ElementEventMap
                    : Target extends AbortSignal
                      ? AbortSignalEventMap
                      : Target extends AbstractWorker
                        ? AbstractWorkerEventMap
                        : Target extends Animation
                          ? AnimationEventMap
                          : Target extends BroadcastChannel
                            ? BroadcastChannelEventMap
                            : Target extends FileReader
                              ? FileReaderEventMap
                              : Target extends IDBDatabase
                                ? IDBDatabaseEventMap
                                : Target extends IDBOpenDBRequest
                                  ? IDBOpenDBRequestEventMap
                                  : Target extends IDBRequest
                                    ? IDBRequestEventMap
                                    : Target extends IDBTransaction
                                      ? IDBTransactionEventMap
                                      : Target extends MediaDevices
                                        ? MediaDevicesEventMap
                                        : Target extends MediaKeySession
                                          ? MediaKeySessionEventMap
                                          : Target extends MediaQueryList
                                            ? MediaQueryListEventMap
                                            : Target extends MediaRecorder
                                              ? MediaRecorderEventMap
                                              : Target extends MediaSource
                                                ? MediaSourceEventMap
                                                : Target extends MediaStream
                                                  ? MediaStreamEventMap
                                                  : Target extends MediaStreamTrack
                                                    ? MediaStreamTrackEventMap
                                                    : Target extends MessagePort
                                                      ? MessagePortEventMap
                                                      : Target extends Notification
                                                        ? NotificationEventMap
                                                        : Target extends PaymentRequest
                                                          ? PaymentRequestEventMap
                                                          : Target extends Performance
                                                            ? PerformanceEventMap
                                                            : Target extends PermissionStatus
                                                              ? PermissionStatusEventMap
                                                              : Target extends PictureInPictureWindow
                                                                ? PictureInPictureWindowEventMap
                                                                : Target extends RemotePlayback
                                                                  ? RemotePlaybackEventMap
                                                                  : Target extends ScreenOrientation
                                                                    ? ScreenOrientationEventMap
                                                                    : Target extends ServiceWorker
                                                                      ? ServiceWorkerEventMap
                                                                      : Target extends ServiceWorkerContainer
                                                                        ? ServiceWorkerContainerEventMap
                                                                        : Target extends ServiceWorkerRegistration
                                                                          ? ServiceWorkerRegistrationEventMap
                                                                          : Target extends ShadowRoot
                                                                            ? ShadowRootEventMap
                                                                            : Target extends SourceBuffer
                                                                              ? SourceBufferEventMap
                                                                              : Target extends SourceBufferList
                                                                                ? SourceBufferListEventMap
                                                                                : Target extends SpeechSynthesis
                                                                                  ? SpeechSynthesisEventMap
                                                                                  : Target extends SpeechSynthesisUtterance
                                                                                    ? SpeechSynthesisUtteranceEventMap
                                                                                    : Target extends VisualViewport
                                                                                      ? VisualViewportEventMap
                                                                                      : Target extends WebSocket
                                                                                        ? WebSocketEventMap
                                                                                        : Target extends Worker
                                                                                          ? WorkerEventMap
                                                                                          : Target extends XMLHttpRequest
                                                                                            ? XMLHttpRequestEventMap
                                                                                            : Target extends XMLHttpRequestUpload
                                                                                              ? XMLHttpRequestEventTargetEventMap
                                                                                              : Target extends EventSource
                                                                                                ? EventSourceEventMap
                                                                                                : never;

/** The event type, handler and options the `eventListener` ref factory listens with. */
export type EventListenerDirectiveProperties<Event_ extends Event = Event> = [
    type: string,
    handler: (event: Event_) => void,
    options?: EventListenerOptions,
];

/** The handlers of each event of an event map. */
export type EventHandlersMap<EventMap> = {
    [EventName in keyof EventMap]: (event: EventMap[EventName]) => void;
};

/** Listen to one more event of a stack, returning how to stop it alone. */
export type EventListenerStackOn<EventMap extends object> = <EventType extends keyof EventMap>(
    type: EventType,
    handler: (event: EventMap[EventType]) => void,
    options?: EventListenerOptions,
) => () => void;

/** The `window` events a `WindowEventListener` handles, as `on` properties. */
export type WindowEventProperties = {
    [Key in keyof WindowEventMap as `on${Capitalize<Key>}` | `on${Key}`]?: (
        event: WindowEventMap[Key],
    ) => void;
};

/** The `document` events a `DocumentEventListener` handles, as `on` properties. */
export type DocumentEventProperties = {
    [Key in keyof DocumentEventMap as `on${Capitalize<Key>}` | `on${Key}`]?: (
        event: DocumentEventMap[Key],
    ) => void;
};

/** Listen to a target's event until the owner is cleaned up or the returned function is called. */
export function makeEventListener<
    Target extends TargetWithEventMap,
    EventMap extends EventMapOf<Target>,
    EventType extends keyof EventMap,
>(
    target: Target,
    type: EventType,
    handler: (event: EventMap[EventType]) => void,
    options?: EventListenerOptions,
): () => void;
/** Listen to a custom event of a target until the owner is cleaned up or the returned function is called. */
export function makeEventListener<
    EventMap extends Record<string, Event>,
    EventType extends keyof EventMap = keyof EventMap,
>(
    target: EventTarget,
    type: EventType,
    handler: (event: EventMap[EventType]) => void,
    options?: EventListenerOptions,
): () => void;
/** Listen to an event of a target, stopping with the owner. */
export function makeEventListener(
    target: EventTarget,
    type: string,
    handler: (event: Event) => void,
    options?: EventListenerOptions,
): () => void {
    // listen, and stop with the owner when there is one
    target.addEventListener(type, handler, options);
    const stop = (): void => target.removeEventListener(type, handler, options);
    if (getOwner() !== null) {
        onCleanup(stop);
    }

    return stop;
}

/** Listen to events of one or several targets, following the targets and types as they change. */
export function createEventListener<
    Target extends TargetWithEventMap,
    EventMap extends EventMapOf<Target>,
    EventType extends keyof EventMap,
>(
    target: MaybeAccessor<Many<Target | undefined>>,
    type: MaybeAccessor<Many<EventType>>,
    handler: (event: EventMap[EventType]) => void,
    options?: EventListenerOptions,
): void;
/** Listen to custom events of one or several targets, following the targets and types as they change. */
export function createEventListener<
    EventMap extends Record<string, Event>,
    EventType extends keyof EventMap = keyof EventMap,
>(
    target: MaybeAccessor<Many<EventTarget | undefined>>,
    type: MaybeAccessor<Many<EventType>>,
    handler: (event: EventMap[EventType]) => void,
    options?: EventListenerOptions,
): void;
/** Listen to the events of the current targets, again whenever they change. */
export function createEventListener(
    targets: MaybeAccessor<Many<EventTarget | undefined>>,
    type: MaybeAccessor<Many<string>>,
    handler: (event: Event) => void,
    options?: EventListenerOptions,
): void {
    // listen nowhere on the server
    if (isServer) {
        return;
    }

    // add the listeners to every target for every type, and remove them before the next run
    const compute = (): { targets: EventTarget[]; types: string[] } => ({
        targets: asArray(access(targets)).filter((target) => target !== undefined),
        types: asArray(access(type)),
    });
    const apply = (current: { targets: EventTarget[]; types: string[] }): (() => void) => {
        for (const target of current.targets) {
            for (const name of current.types) {
                target.addEventListener(name, handler, options);
            }
        }

        return () => {
            for (const target of current.targets) {
                for (const name of current.types) {
                    target.removeEventListener(name, handler, options);
                }
            }
        };
    };

    // wait for mount when the targets are an accessor, as refs are, else listen right away
    if (typeof targets === "function") {
        createEffect(compute, apply, TRANSPARENT);
    } else {
        createRenderEffect(compute, apply, TRANSPARENT);
    }
}

/** Follow the latest event of one or several targets, undefined until the first. */
export function createEventSignal<
    Target extends TargetWithEventMap,
    EventMap extends EventMapOf<Target>,
    EventType extends keyof EventMap,
>(
    target: MaybeAccessor<Many<Target>>,
    type: MaybeAccessor<Many<EventType>>,
    options?: EventListenerOptions,
): Accessor<EventMap[EventType] | undefined>;
/** Follow the latest custom event of one or several targets, undefined until the first. */
export function createEventSignal<
    EventMap extends Record<string, Event>,
    EventType extends keyof EventMap = keyof EventMap,
>(
    target: MaybeAccessor<Many<EventTarget>>,
    type: MaybeAccessor<Many<EventType>>,
    options?: EventListenerOptions,
): Accessor<EventMap[EventType] | undefined>;
/**
 * Hold each event the listener receives.
 *
 * @construct the accessor reads the last event of the listened types, which the event map types
 */
export function createEventSignal(
    target: MaybeAccessor<Many<EventTarget>>,
    type: MaybeAccessor<Many<string>>,
    options?: EventListenerOptions,
): Accessor<Event | undefined> {
    // hold nothing on the server
    if (isServer) {
        return () => undefined;
    }
    const [event, setEvent] = createSignal<Event | undefined>(undefined, { ownedWrite: true });
    createEventListener(target, type, (latest: Event) => setEvent(latest), options);

    return event;
}

/** Listen to an element's event from its ref, following reactive properties. */
export function eventListener<Event_ extends Event = Event>(
    properties: MaybeAccessor<EventListenerDirectiveProperties<Event_>>,
): (target: EventTarget) => void {
    // listen nowhere on the server
    if (isServer) {
        return () => {};
    }

    // listen on the element the ref receives, again whenever the properties change
    const [target, setTarget] = createSignal<EventTarget | undefined>(undefined, {
        ownedWrite: true,
    });
    createEffect(
        () => ({ element: target(), listened: access(properties) }),
        ({ element, listened }) => {
            if (element === undefined) {
                return undefined;
            }
            const [type, handler, options] = listened;
            const listener: EventListenerObject = { handleEvent: handler };
            element.addEventListener(type, listener, options);

            return () => element.removeEventListener(type, listener, options);
        },
        TRANSPARENT,
    );

    return (element) => {
        setTarget(element);
    };
}

/** Listen to several events of one or several targets, each with its own handler. */
export function createEventListenerMap<
    Target extends TargetWithEventMap,
    EventMap extends EventMapOf<Target>,
>(
    target: MaybeAccessor<Many<Target>>,
    handlersMap: Partial<EventHandlersMap<EventMap>>,
    options?: EventListenerOptions,
): void;
/** Listen to several custom events of one or several targets, each with its own handler. */
export function createEventListenerMap<EventMap extends Record<string, Event>>(
    target: MaybeAccessor<Many<EventTarget>>,
    handlersMap: Partial<EventHandlersMap<EventMap>>,
    options?: EventListenerOptions,
): void;
/** Listen to each event of the map with its handler. */
export function createEventListenerMap(
    targets: MaybeAccessor<Many<EventTarget>>,
    handlersMap: Record<string, ((event: Event) => void) | undefined>,
    options?: EventListenerOptions,
): void {
    for (const [type, handler] of Object.entries(handlersMap)) {
        if (handler !== undefined) {
            createEventListener(targets, type, handler, options);
        }
    }
}

/** Collect listeners on one target, all stopped together on cleanup or by the returned function. */
export function makeEventListenerStack<
    Target extends TargetWithEventMap,
    EventMap extends EventMapOf<Target>,
>(
    target: Target,
    options?: EventListenerOptions,
): [listen: EventListenerStackOn<EventMap>, clear: () => void];
/** Collect listeners to custom events on one target, all stopped together on cleanup or by the returned function. */
export function makeEventListenerStack<EventMap extends Record<string, Event>>(
    target: EventTarget,
    options?: EventListenerOptions,
): [listen: EventListenerStackOn<EventMap>, clear: () => void];
/**
 * Collect the listeners of one target.
 *
 * @construct each listener's handler takes the event its type names in the target's event map
 */
export function makeEventListenerStack(
    target: EventTarget,
    options?: EventListenerOptions,
): [listen: EventListenerStackOn<Record<string, Event>>, clear: () => void] {
    // listen nowhere on the server
    if (isServer) {
        return [() => () => {}, () => {}];
    }

    // stop every listener of the stack at once
    let stops: (() => void)[] = [];
    const clear = (): void => {
        for (const stop of stops) {
            stop();
        }
        stops = [];
    };
    if (getOwner() !== null) {
        onCleanup(clear);
    }

    return [
        (type, handler, overwriteOptions) => {
            const stop = makeEventListener(target, type, handler, overwriteOptions ?? options);
            stops.push(stop);

            return stop;
        },
        clear,
    ];
}

/** Listen to `window` events through `on` properties. */
export const WindowEventListener: Component<WindowEventProperties> = (properties) => {
    if (!isServer) {
        listenToProperties(window, properties);
    }

    return undefined;
};

/** Listen to `document` events through `on` properties. */
export const DocumentEventListener: Component<DocumentEventProperties> = (properties) => {
    if (!isServer) {
        listenToProperties(document, properties);
    }

    return undefined;
};

/** Wrap a handler to prevent the event's default action first. */
export function preventDefault<Event_ extends Event>(
    callback: (event: Event_) => void,
): (event: Event_) => void {
    return (event) => {
        event.preventDefault();
        callback(event);
    };
}

/** Wrap a handler to stop the event's propagation first. */
export function stopPropagation<Event_ extends Event>(
    callback: (event: Event_) => void,
): (event: Event_) => void {
    return (event) => {
        event.stopPropagation();
        callback(event);
    };
}

/** Wrap a handler to stop the event reaching any other listener first. */
export function stopImmediatePropagation<Event_ extends Event>(
    callback: (event: Event_) => void,
): (event: Event_) => void {
    return (event) => {
        event.stopImmediatePropagation();
        callback(event);
    };
}

/** Listen to a target's events through `on` properties, naming each event by the rest of the property in lower case. */
function listenToProperties(target: EventTarget, properties: object): void {
    for (const key of Object.keys(properties)) {
        const handler: unknown = Reflect.get(properties, key);
        if (key.startsWith("on") && isHandler(handler)) {
            makeEventListener(target, key.slice(2).toLowerCase(), handler);
        }
    }
}

/** Check whether a property holds an event handler. */
function isHandler(value: unknown): value is (event: Event) => void {
    return typeof value === "function";
}

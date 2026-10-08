import { isServer } from "@solidjs/web";

/** The event handlers among a component's properties, by event name: `onChangeStep` as `changeStep`. */
export type Handlers<Properties> = {
    [
        Property in keyof Properties as Property extends `on${infer EventName}`
            ? Uncapitalize<EventName>
            : never
    ]: Properties[Property];
};

/** How a dispatched event behaves. */
export type DispatcherOptions = {
    /** Whether the handler may cancel the event, false by default. */
    readonly cancelable?: boolean;
};

/** The arguments of a dispatch: the event name, its payload, typed by the handler, and the options. */
export type DispatchArguments<
    Properties,
    Name extends keyof Handlers<Properties>,
> = Handlers<Properties>[Name] extends undefined | ((event?: CustomEvent<infer Detail>) => unknown)
    ? [eventName: Name, payload?: Detail, options?: DispatcherOptions]
    : Handlers<Properties>[Name] extends undefined | ((event: CustomEvent<infer Detail>) => unknown)
      ? [eventName: Name, payload: Detail, options?: DispatcherOptions]
      : [eventName: Name, payload?: unknown, options?: DispatcherOptions];

/** Dispatch typed custom events to a component's `on` handlers, reporting whether none was cancelled. */
export function createEventDispatcher<Properties>(
    properties: Properties,
): <Name extends keyof Handlers<Properties> & string>(
    ...args: DispatchArguments<Properties, Name>
) => boolean;
/**
 * Call the handler an event name selects with a custom event of the payload.
 *
 * @construct an event name selects the `on` handler of the same name, which takes the payload its type gives
 */
export function createEventDispatcher(
    properties: object,
): (eventName: string, payload?: unknown, options?: DispatcherOptions) => boolean {
    // dispatch nothing on the server
    if (isServer) {
        return () => true;
    }

    return (eventName, payload, options) => {
        // find the handler, dispatching nothing without one
        const handler: unknown = Reflect.get(
            properties,
            `on${eventName.charAt(0).toUpperCase()}${eventName.slice(1)}`,
        );
        if (!isHandler(handler)) {
            return true;
        }

        // call it with the event, reporting whether it went uncancelled
        const event = new CustomEvent(eventName, {
            detail: payload,
            cancelable: options?.cancelable ?? false,
        });
        handler(event);

        return !event.defaultPrevented;
    };
}

/** Check whether a property holds an event handler. */
function isHandler(value: unknown): value is (event: CustomEvent) => void {
    return typeof value === "function";
}

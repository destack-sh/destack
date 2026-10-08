import { createSignal } from "solid-js";

/** The name of an event an HTML element dispatches. */
export type HTMLEventName = keyof HTMLElementEventMap;

/** One or more HTML event names. */
export type EventNames = [name: HTMLEventName, ...moreNames: HTMLEventName[]];

/** The latest event of each name, absent until the first. */
export type EventStore<Names extends HTMLEventName[]> = {
    [Name in Names[number]]?: HTMLElementEventMap[Name];
};

/** The `on` properties that record each named event. */
export type EventProperties<Names extends EventNames> = {
    [Name in Names[number] as `on${Name}`]: (event: HTMLElementEventMap[Name]) => void;
};

/** Make `on` properties for an element's events and a store of the latest one of each. */
export function createEventProps<Names extends EventNames>(
    ...names: Names
): [EventStore<Names>, EventProperties<Names>];
/**
 * Record each named event in a signal of its own.
 *
 * @construct each name gets a store getter and an `on` property of that name, which is how the mapped types read them
 */
export function createEventProps(
    ...names: string[]
): [object, Record<string, (event: Event) => void>] {
    // hold each event in a signal behind a store getter and an on property
    const store = {};
    const properties: Record<string, (event: Event) => void> = {};
    for (const name of names) {
        const [event, setEvent] = createSignal<Event | undefined>(undefined, { ownedWrite: true });
        const record = (latest: Event): void => {
            setEvent(latest);
        };
        Object.defineProperty(store, name, { enumerable: true, get: event, set: record });
        properties[`on${name}`] = record;
    }

    return [store, properties];
}

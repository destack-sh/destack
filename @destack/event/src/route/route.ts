import type { Address, Destination } from "@destack/service/outbox";
import { Event, type EventKind } from "../kind/kind.ts";

/** The events one delivery of routed events sends at most. */
const ROUTE_BATCH = 100;

/** The outbox address of a kind's events routed to other scopes. */
export function routeAddress(kind: EventKind): Address<Event> {
    return { name: `event ${kind.key}`, message: Event };
}

/** The outbox destination delivering a kind's routed events in order. */
export function routeDestination(
    kind: EventKind,
    deliver: (kind: string, events: readonly Event[], signal: AbortSignal) => Promise<void>,
): Destination<Event> {
    return {
        ...routeAddress(kind),
        batch: ROUTE_BATCH,
        accept: (events, { signal }) => deliver(kind.key, events, signal),
    };
}

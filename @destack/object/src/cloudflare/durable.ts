import type { Clock } from "@destack/environment";
import type { Alarm } from "@destack/db";

/** How long an alarm runs a Durable Object's controllers at most: 10 minutes. */
export const ALARM_MILLISECONDS = 10 * 60_000;

/** The members of a Durable Object's state a workload object uses. */
export interface DurableObjectState {
    /** The object's storage, whose alarm wakes the object. */
    readonly storage: Alarm;
    /** Block the object's other events until a closure settles. */
    readonly blockConcurrencyWhile: <Value>(closure: () => Promise<Value>) => Promise<Value>;
}

/** A workload an object serves: its requests, and the controllers the object's alarm wakes. */
export interface DurableObjectWorkloadInstance {
    /** Serve a request. */
    readonly fetch: (request: Request) => Promise<Response>;
    /** Run the controllers until no key is due now or reconciling, or until a deadline. */
    readonly alarm: (deadline: number) => Promise<void>;
}

/** A Durable Object with one workload instance. */
export class DurableObjectWorkload {
    /** The started workload. */
    readonly #started: Promise<DurableObjectWorkloadInstance>;
    /** The clock alarms run on. */
    readonly #clock: Clock;

    /** Start a workload before the object takes any event, its alarms timed on a clock. */
    constructor(
        state: DurableObjectState,
        start: (alarm: Alarm) => Promise<DurableObjectWorkloadInstance>,
        clock: Clock,
    ) {
        this.#started = state.blockConcurrencyWhile(() => start(state.storage));
        this.#clock = clock;
    }

    /** Serve a request through the started workload. */
    async fetch(request: Request): Promise<Response> {
        const workload = await this.#started;

        return workload.fetch(request);
    }

    /** Run the controllers due at the alarm. */
    async alarm(): Promise<void> {
        // run for a bounded time
        const workload = await this.#started;
        await workload.alarm(this.#clock.now() + ALARM_MILLISECONDS);
    }
}

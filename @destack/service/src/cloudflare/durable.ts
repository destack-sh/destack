import type { Alarm } from "../control/index.ts";

/** How long an alarm runs the controllers at most: 10 minutes. */
const ALARM_MILLISECONDS = 10 * 60_000;

/** The members of a Durable Object's state a workload object uses. */
export interface DurableObjectState {
    /** The object's storage, whose alarm wakes the object. */
    readonly storage: Alarm;
    /** Block the object's other events until a closure settles. */
    blockConcurrencyWhile<Value>(closure: () => Promise<Value>): Promise<Value>;
}

/** A workload an object serves: its requests, and the controllers the object's alarm wakes. */
export interface DurableObjectWorkloadInstance {
    /** Serve a request. */
    fetch(request: Request): Promise<Response>;
    /** Run the controllers until no key is due now or reconciling, or until a deadline. */
    alarm(deadline: number): Promise<void>;
}

/** A Durable Object with one workload instance. */
export class DurableObjectWorkload {
    /** The started workload. */
    readonly #started: Promise<DurableObjectWorkloadInstance>;

    /** Start a workload before the object takes any event. */
    constructor(
        state: DurableObjectState,
        start: (alarm: Alarm) => Promise<DurableObjectWorkloadInstance>,
    ) {
        this.#started = state.blockConcurrencyWhile(() => start(state.storage));
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
        await workload.alarm(Date.now() + ALARM_MILLISECONDS);
    }
}

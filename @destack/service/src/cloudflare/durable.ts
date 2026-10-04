import type { Alarm } from "../control/index.ts";

/** How long an alarm runs the controllers at most: 10 minutes, below the 15 minutes a Durable Object's alarm may run. */
const ALARM_MILLISECONDS = 10 * 60_000;

/** The members of a Durable Object's state a workload object uses: the storage keeping its alarm, and the gate holding its events while it starts. */
export interface DurableState {
    /** The object's storage, whose alarm wakes the object. */
    readonly storage: Alarm;
    /** Hold the object's other events until a closure settles, resetting the object when it fails. */
    blockConcurrencyWhile<Value>(closure: () => Promise<Value>): Promise<Value>;
}

/** A workload an object serves: its requests, and the controllers the object's alarm wakes. */
export interface DurableInstance {
    /** Serve a request. */
    fetch(request: Request): Promise<Response>;
    /** Run the controllers until no key is due now or reconciling, or until a deadline, keeping the alarm due while keys still run. */
    alarm(deadline: number): Promise<void>;
}

/** A Durable Object holding one workload instance: started before its first event, its controllers woken by the object's alarm. */
export class DurableWorkload {
    /** The started workload. */
    readonly #started: Promise<DurableInstance>;

    /** Start a workload before the object takes any event, keeping its controllers' wake-up on the object's alarm. */
    constructor(state: DurableState, start: (alarm: Alarm) => Promise<DurableInstance>) {
        this.#started = state.blockConcurrencyWhile(() => start(state.storage));
    }

    /** Serve a request through the started workload. */
    async fetch(request: Request): Promise<Response> {
        const workload = await this.#started;

        return workload.fetch(request);
    }

    /** Run the controllers due at the alarm, settling once none is due or at the alarm's deadline. */
    async alarm(): Promise<void> {
        // run for a bounded time, keeping the alarm due so a follow resumes across alarms
        const workload = await this.#started;
        await workload.alarm(Date.now() + ALARM_MILLISECONDS);
    }
}

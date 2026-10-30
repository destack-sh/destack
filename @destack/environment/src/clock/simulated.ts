import { Clock, LONGEST_DELAY } from "./clock.ts";

/** A wait on a simulated clock, released once the clock reaches its deadline. */
interface SimulatedSleep {
    /** When the wait ends, in UTC epoch milliseconds. */
    readonly deadline: number;
    /** The order the wait began in, which releases waits of one deadline first come first. */
    readonly order: number;
    /** Release the wait. */
    readonly release: () => void;
    /** Fail the wait with a failure of the world it waits in. */
    readonly fail: (error: unknown) => void;
    /** Whether the wait keeps the process running: a sleep does, a timeout signal does not. */
    readonly isHeld: boolean;
    /** Whether the wait was withdrawn before its deadline. */
    isWithdrawn: boolean;
}

/** A clock that a test pauses, moves and resumes. */
export class SimulatedClock implements Clock {
    /** The clock's reading when it paused or last moved, in UTC epoch milliseconds. */
    #time: number;
    /** The system's time when the clock resumed or last moved, absent while it is paused. */
    #resumedAt: number | undefined;
    /** The system timer that wakes the earliest wait while the clock flows. */
    #timer: ReturnType<typeof setTimeout> | undefined;
    /** The pending waits as a binary heap, earliest deadline first, withdrawn ones skipped. */
    readonly #sleeps: SimulatedSleep[] = [];
    /** The pending waits not withdrawn that stop the process from exiting. */
    #held = 0;
    /** The order of the next wait. */
    #order = 0;
    /** The waits begun so far. */
    #begun = 0;
    /** The waiters for the next wait to begin. */
    #activity: (() => void)[] = [];
    /** The stops of the pending timers, by handle. */
    readonly #timers = new Map<number, AbortController>();
    /** The next timer handle. */
    #handle = 1;
    /** The call sites of the waits begun while recording, absent between recordings. */
    #recorded: string[] | undefined;
    /** Whether a world's loop alone moves the clock, refusing anyone else's move. */
    #isOwned = false;
    /** The failure of the loop that moves the clock, refusing every later wait. */
    #failure: { readonly error: unknown } | undefined;

    /** Start the clock paused at a time. */
    constructor(time: number) {
        this.#time = time;
    }

    /** View this clock from a node, off by an offset and running at a rate, from now on. */
    view(skew: { readonly offset: number; readonly rate: number }): Clock & {
        readonly bump: (milliseconds: number) => void;
    } {
        return new SimulatedClockView(this, skew.offset, skew.rate);
    }

    /** Make a clock a world's loop alone moves, with the move only that loop gets. */
    static owned(time: number): {
        readonly clock: SimulatedClock;
        readonly move: (time: number) => void;
    } {
        // refuse every move but the loop's
        const clock = new SimulatedClock(time);
        clock.#isOwned = true;

        return { clock, move: (to) => clock.#move(to, Clock.system.now()) };
    }

    /** Read the clock's time. */
    now(): number {
        return this.#at(Clock.system.now());
    }

    /** Wait some milliseconds from now, rounded up to whole ones, until the clock passes them. */
    sleep(milliseconds: number, signal?: AbortSignal): Promise<void> {
        // count the wait, ending an empty one at once
        this.#begun += 1;
        this.#record("sleep");
        if (milliseconds <= 0 && signal?.aborted !== true) {
            return Promise.resolve();
        }

        return this.#wait(milliseconds, true, signal);
    }

    /** Make a signal aborting with a timeout once the clock passes some milliseconds from now. */
    timeout(milliseconds: number): AbortSignal {
        // abort as the web's timeout signal does, outside the counted waits
        const controller = new AbortController();
        void this.#wait(milliseconds, false).then(() =>
            controller.abort(new DOMException("signal timed out", "TimeoutError")),
        );

        return controller.signal;
    }

    /** Add a wait until the clock passes some milliseconds from now, a zero wait due at once. */
    #wait(milliseconds: number, isHeld: boolean, signal?: AbortSignal): Promise<void> {
        // refuse aborted waits, and every wait once the loop failed
        if (signal?.aborted === true) {
            return Promise.reject(signal.reason);
        } else if (this.#failure !== undefined) {
            return Promise.reject(this.#failure.error);
        }

        // add the wait until its deadline
        const { promise, resolve, reject } = Promise.withResolvers<void>();
        const sleep: SimulatedSleep = {
            deadline: this.now() + Math.max(Math.ceil(milliseconds), 0),
            order: this.#order,
            release: () => {
                signal?.removeEventListener("abort", abort);
                resolve();
            },
            fail: (error) => {
                signal?.removeEventListener("abort", abort);
                reject(error);
            },
            isHeld,
            isWithdrawn: false,
        };
        const abort = () => {
            // withdraw the wait, and follow the next one
            sleep.isWithdrawn = true;
            this.#held -= isHeld ? 1 : 0;
            this.#follow();
            reject(signal?.reason);
        };
        this.#order += 1;
        this.#held += isHeld ? 1 : 0;
        push(this.#sleeps, sleep);
        signal?.addEventListener("abort", abort, { once: true });
        this.#wakeActivity();

        // wake it on time while the clock flows
        this.#follow();

        return promise;
    }

    /** Wait until a wait begins. */
    nextActivity(): Promise<void> {
        return new Promise<void>((resolve) => {
            this.#activity.push(resolve);
        });
    }

    /** Let anyone move the clock once the loop that moved it stopped. */
    detach(): void {
        this.#isOwned = false;
    }

    /** Fail every pending wait the process runs for, counting them. */
    fail(error: unknown): number {
        // remember the failure for later waits
        this.#failure = { error };

        // fail the waits the process runs for, dropping timeout signals
        let failed = 0;
        for (let sleep = pop(this.#sleeps); sleep !== undefined; sleep = pop(this.#sleeps)) {
            if (!sleep.isWithdrawn && sleep.isHeld) {
                sleep.fail(error);
                failed += 1;
            }
        }

        return failed;
    }

    /** Record the call sites of the waits begun until the returned stop. */
    record(): () => readonly string[] {
        const recorded: string[] = [];
        this.#recorded = recorded;

        return () => {
            this.#recorded = undefined;

            return recorded;
        };
    }

    /** Wake each waiter for the next wait to begin. */
    #wakeActivity(): void {
        for (const resolve of this.#activity.splice(0)) {
            resolve();
        }
    }

    /** Note a wait's call site while recording. */
    #record(kind: string): void {
        // read the call site from the stack while recording
        if (this.#recorded === undefined) {
            return;
        }
        const { stack } = new Error();
        if (stack === undefined) {
            throw new TypeError("the runtime records no call stacks");
        }
        this.#recorded.push(`${kind} ${stack.split("\n").slice(3, 9).join(" < ")}`);
    }

    /** The waits begun so far. */
    get begun(): number {
        return this.#begun;
    }

    /** The earliest deadline of a pending wait, absent with none. */
    get next(): number | undefined {
        // drop withdrawn waits from the top
        while (this.#sleeps[0]?.isWithdrawn === true) {
            pop(this.#sleeps);
        }

        return this.#sleeps[0]?.deadline;
    }

    /** Let the clock flow with the system's time from its current reading. */
    resume(): this {
        this.#refuseOwned();
        if (this.#resumedAt === undefined) {
            this.#resumedAt = Clock.system.now();
            this.#follow();
        }

        return this;
    }

    /** Stop the clock at its current reading. */
    pause(): void {
        // stop at the reading, releasing the waits it already passed
        this.#time = this.now();
        this.#resumedAt = undefined;
        this.#wake();
    }

    /** Move the clock to a later time, releasing the waits it passes, earliest first. */
    set(time: number): void {
        this.#refuseOwned();
        this.#move(time, Clock.system.now());
    }

    /** Move the clock forward some milliseconds, releasing the waits it passes. */
    advance(milliseconds: number): void {
        this.#refuseOwned();
        const system = Clock.system.now();
        this.#move(this.#at(system) + milliseconds, system);
    }

    /** Refuse moving a clock a world's loop alone moves. */
    #refuseOwned(): void {
        if (this.#isOwned) {
            throw new Error(
                "a world's clock moves by its loop: jump it, or sleep and wait for idle",
            );
        }
    }

    /** Read the clock at a reading of the system's clock. */
    #at(system: number): number {
        return this.#resumedAt === undefined ? this.#time : this.#time + system - this.#resumedAt;
    }

    /** Move the clock to a later time at one reading of the system's clock. */
    #move(time: number, system: number): void {
        // refuse moving back, and a time between whole milliseconds
        const now = this.#at(system);
        if (time < now) {
            throw new RangeError(`a simulated clock at ${now} cannot go back to ${time}`);
        } else if (!Number.isInteger(time)) {
            throw new RangeError(`a simulated clock moves in whole milliseconds, not to ${time}`);
        }

        // move the reading, keeping a flowing clock flowing
        this.#time = time;
        if (this.#resumedAt !== undefined) {
            this.#resumedAt = system;
        }
        this.#wake();
    }

    /** Run a function once some milliseconds of the clock passed, answering the timer's handle. */
    setTimeout(handler: unknown, delay = 0, ...values: unknown[]): number {
        return this.#schedule(handler, delay, values, false);
    }

    /** Run a function each period of the clock, answering the timer's handle. */
    setInterval(handler: unknown, delay = 0, ...values: unknown[]): number {
        return this.#schedule(handler, delay, values, true);
    }

    /** Stop a timer. */
    clearTimeout(handle?: number): void {
        if (handle !== undefined) {
            this.#timers.get(handle)?.abort();
            this.#timers.delete(handle);
        }
    }

    /** Stop an interval. */
    clearInterval(handle?: number): void {
        this.clearTimeout(handle);
    }

    /** Start a timer under a new handle. */
    #schedule(handler: unknown, delay: number, values: unknown[], isRepeating: boolean): number {
        // refuse code strings
        if (typeof handler !== "function") {
            throw new TypeError("a simulated timer can only run functions");
        }

        // add the timer under a new handle
        const handle = this.#handle;
        this.#handle += 1;
        const stop = new AbortController();
        this.#timers.set(handle, stop);

        // run a delay past the longest one after a millisecond, as Node's and Bun's timers overflow
        const bounded = delay > LONGEST_DELAY ? 1 : delay;

        // run a single timer of no time due at once and an interval each millisecond at least
        const period = isRepeating ? Math.max(bounded, 1) : Math.max(bounded, 0);
        this.#arm(handle, handler, period, values, isRepeating, stop.signal);

        return handle;
    }

    /** Wait one period of a timer, then run it. */
    #arm(
        handle: number,
        handler: Function,
        delay: number,
        values: unknown[],
        isRepeating: boolean,
        signal: AbortSignal,
    ): void {
        this.#begun += 1;
        this.#wait(delay, true, signal).then(
            () => {
                // wait again for an interval
                if (isRepeating) {
                    this.#arm(handle, handler, delay, values, isRepeating, signal);
                }
                // remove a single timer
                else {
                    this.#timers.delete(handle);
                }
                Reflect.apply(handler, undefined, values);
            },
            () => {},
        );
    }

    /** Release the waits the clock reached, earliest first. */
    #wake(): void {
        // release the due waits in deadline order, first come first within a deadline
        const now = this.now();
        while (this.next !== undefined && this.next <= now) {
            const sleep = pop(this.#sleeps);
            this.#held -= sleep?.isHeld === true ? 1 : 0;
            sleep?.release();
        }

        // follow the next wait
        this.#follow();
    }

    /** Set a system timer for the earliest wait while the clock flows. */
    #follow(): void {
        // clear the earlier timer
        clearTimeout(this.#timer);
        this.#timer = undefined;

        // wake at the earliest deadline while flowing, keeping the process running for held waits alone
        const next = this.next;
        if (this.#resumedAt !== undefined && next !== undefined) {
            const timer = setTimeout(() => this.#wake(), Math.max(next - this.now(), 0));
            if (this.#held === 0) {
                timer.unref();
            }
            this.#timer = timer;
        }
    }
}

/** A node's view of a world's clock: off by an offset and running at a rate of its drift. */
class SimulatedClockView implements Clock {
    /** The world's clock, whose timers every wait runs on. */
    readonly #world: SimulatedClock;
    /** The world's time the drift counts from. */
    readonly #anchor: number;
    /** The node's milliseconds per world millisecond, one without drift. */
    readonly rate: number;
    /** The milliseconds the node's clock is ahead of the world's, behind when negative. */
    #offset: number;

    /** View a world's clock off by an offset and running at a rate from now on. */
    constructor(world: SimulatedClock, offset: number, rate: number) {
        // anchor the drift at the world's time now
        this.#world = world;
        this.#anchor = world.now();
        this.#offset = offset;
        this.rate = rate;
    }

    /** Jump the node's clock by some milliseconds, back when negative, as a clock fault does. */
    bump(milliseconds: number): void {
        this.#offset += milliseconds;
    }

    /** Read the node's time: the world's, off by the offset and drifted since the anchor. */
    now(): number {
        const world = this.#world.now();

        return world + this.#offset + Math.floor((world - this.#anchor) * (this.rate - 1));
    }

    /** Wait some of the node's milliseconds on the world's timers. */
    sleep(milliseconds: number, signal?: AbortSignal): Promise<void> {
        return this.#world.sleep(milliseconds / this.rate, signal);
    }

    /** Make a signal aborting once some of the node's milliseconds pass. */
    timeout(milliseconds: number): AbortSignal {
        return this.#world.timeout(milliseconds / this.rate);
    }

    /** Run a function once some of the node's milliseconds pass. */
    setTimeout(handler: () => void, delay = 0): number {
        return this.#world.setTimeout(handler, delay / this.rate);
    }

    /** Run a function each period of the node's milliseconds. */
    setInterval(handler: () => void, delay = 0): number {
        return this.#world.setInterval(handler, delay / this.rate);
    }

    /** Stop a timer by its handle. */
    clearTimeout(handle?: number): void {
        this.#world.clearTimeout(handle);
    }

    /** Stop an interval by its handle. */
    clearInterval(handle?: number): void {
        this.#world.clearInterval(handle);
    }
}

/** Report whether a wait comes before another: an earlier deadline, else an earlier start. */
function precedes(left: SimulatedSleep, right: SimulatedSleep): boolean {
    return (
        left.deadline < right.deadline ||
        (left.deadline === right.deadline && left.order < right.order)
    );
}

/** Add a wait to a binary heap, moving it up past the waits it precedes. */
function push(heap: SimulatedSleep[], sleep: SimulatedSleep): void {
    // append it, then swap it with each parent it precedes
    heap.push(sleep);
    let index = heap.length - 1;
    while (index > 0) {
        const parentIndex = (index - 1) >> 1;
        const parent = heap[parentIndex];
        if (parent === undefined || !precedes(sleep, parent)) {
            return;
        }
        heap[parentIndex] = sleep;
        heap[index] = parent;
        index = parentIndex;
    }
}

/** Remove the earliest wait from a binary heap, moving the last one down into place. */
function pop(heap: SimulatedSleep[]): SimulatedSleep | undefined {
    // take the top, and move the last wait to it
    const top = heap[0];
    const last = heap.pop();
    if (top === undefined || last === undefined || heap.length === 0) {
        return top;
    }
    heap[0] = last;

    // swap the moved wait with its earlier child until both children come after it
    let index = 0;
    for (;;) {
        const left = heap[2 * index + 1];
        const right = heap[2 * index + 2];
        const child =
            right !== undefined && left !== undefined && precedes(right, left) ? right : left;
        if (child === undefined || !precedes(child, last)) {
            return top;
        }
        const childIndex = child === left ? 2 * index + 1 : 2 * index + 2;
        heap[index] = child;
        heap[childIndex] = last;
        index = childIndex;
    }
}

import { SpanStatusCode } from "@opentelemetry/api";
import { yieldToEventLoop } from "../clock/wait.ts";
import type { Telemetry } from "../telemetry/telemetry.ts";
import type { Finisher, Scheduler } from "./scheduler.ts";
import type { Simulation } from "../simulation/simulation.ts";

/** The instrumentation scope of a world's spans and log records. */
export const SIMULATION_SCOPE = "destack.simulation";

/** Work in flight: who finishes it, where it began, and whether its answer waits for the loop. */
interface Flight {
    /** Who finishes it. */
    readonly by: Finisher;
    /** Where it began, as an error whose stack is read only when listed. */
    readonly began: Error;
    /** Whether it settled, its answer waiting for the loop to release it in order. */
    isSettled: boolean;
}

/** Settled host work the loop has not answered yet. */
interface Answer {
    /** The order the work began in. */
    readonly order: number;
    /** Who finished it. */
    readonly by: Finisher;
    /** Answer the work with what it settled with. */
    readonly answer: () => void;
    /** Fail the work with the loop's failure. */
    readonly fail: (error: unknown) => void;
}

/** A world's scheduler: its work in flight, host results answered in the order they began. */
export class SimulatedScheduler implements Scheduler {
    /** Whether a loop answers settled host work in the order it began, instead of at once. */
    #isOrdered: boolean;
    /** The work in flight. */
    readonly #flights = new Set<Flight>();
    /** The host work in flight, peers included. */
    #hosts = 0;
    /** The peer work in flight that has not settled yet. */
    #peers = 0;
    /** The order of the next host work. */
    #sequence = 0;
    /** The settled host work the loop has not answered yet. */
    readonly #answers: Answer[] = [];
    /** The work settled so far. */
    #settled = 0;
    /** The waiters for the next work to begin or settle. */
    #activity: (() => void)[] = [];
    /** The world's failure, refusing every later answer. */
    #failure: { readonly error: unknown } | undefined;
    /** The world's failure as a promise, rejected once the world fails. */
    readonly #failed = Promise.withResolvers<never>();
    /** The call sites of the work begun while recording, absent while not. */
    #recorded: string[] | undefined;
    /** The telemetry each work's span goes to, absent while the world traces nothing. */
    readonly #telemetry: Telemetry | undefined;
    /** The world the scheduler runs, absent for a scheduler outside one. */
    readonly world: Simulation | undefined;

    /** Schedule a world's work, its host results in order when ordered, traced with telemetry. */
    constructor(options: {
        readonly isOrdered: boolean;
        readonly telemetry?: Telemetry;
        readonly world?: Simulation;
    }) {
        // keep the options, leaving the failure unhandled until a wait races it
        this.#isOrdered = options.isOrdered;
        this.#telemetry = options.telemetry;
        this.world = options.world;
        this.#failed.promise.catch(() => undefined);
    }

    /** Read the world scheduler of an environment, refusing one outside a world. */
    static of(environment: { readonly scheduler: Scheduler }): SimulatedScheduler {
        const { scheduler } = environment;
        if (!(scheduler instanceof SimulatedScheduler)) {
            throw new TypeError("the environment runs outside a world");
        }

        return scheduler;
    }

    /** Let the host's event loop run once, tracked as host work. */
    yield(): Promise<void> {
        return this.track(yieldToEventLoop(), "host");
    }

    /** Count work in flight until it settles, answering host work in the order it began. */
    track<Value>(work: Promise<Value>, by: Finisher): Promise<Value> {
        // count world work as it is
        const flight: Flight = { by, began: new Error(by), isSettled: false };
        if (by === "world") {
            return this.#count(work, flight);
        }

        // count a peer's work apart from leaf host work until it settles, which the loop does not wait for
        if (by === "peer") {
            this.#peers += 1;
            const settled = () => {
                this.#peers -= 1;
            };
            void work.then(settled, settled);
        }

        return this.#count(this.#ordered(work, flight), flight);
    }

    /** Answer the settled host work in the order it began, returning each one's order and kind. */
    answer(): { readonly order: number; readonly by: Finisher }[] {
        const answers = this.#answers.splice(0).toSorted((left, right) => left.order - right.order);
        for (const each of answers) {
            each.answer();
        }

        return answers.map(({ order, by }) => ({ order, by }));
    }

    /** Answer host work at once from now on, settled answers first, once no loop orders it. */
    detach(): void {
        this.#isOrdered = false;
        this.answer();
    }

    /** Fail the world with its first failure, refusing settled and later answers, counting them. */
    fail(error: unknown): number {
        // keep the first failure, rejecting its waiters and waking the loop to fail the world
        this.#failure ??= { error };
        this.#failed.reject(this.#failure.error);
        this.#wake();

        // fail the settled answers
        const answers = this.#answers.splice(0);
        for (const each of answers) {
            each.fail(this.#failure.error);
        }

        return answers.length;
    }

    /** The world's failure, absent while it runs. */
    get failure(): { readonly error: unknown } | undefined {
        return this.#failure;
    }

    /** Settles never, rejecting once the world fails, for a wait outside the world to race. */
    get failed(): Promise<never> {
        return this.#failed.promise;
    }

    /** Wait until work begins or settles. */
    nextActivity(): Promise<void> {
        return new Promise<void>((resolve) => {
            this.#activity.push(resolve);
        });
    }

    /** Record the call sites of the work begun until the returned stop. */
    record(): () => readonly string[] {
        const recorded: string[] = [];
        this.#recorded = recorded;

        return () => {
            this.#recorded = undefined;

            return recorded;
        };
    }

    /** List the work in flight: who finishes it, whether it runs or settled, and where it began. */
    pending(): string[] {
        return [...this.#flights].map(
            ({ by, began, isSettled }) =>
                `${by} ${isSettled ? "settled" : "running"} ${SimulatedScheduler.#site(began)}`,
        );
    }

    /** The work in flight. */
    get inFlight(): number {
        return this.#flights.size;
    }

    /** The host work in flight, peers included. */
    get hosts(): number {
        return this.#hosts;
    }

    /** The leaf host work in flight that has not settled yet. */
    get hostPending(): number {
        return this.#hosts - this.#answers.length - this.#peers;
    }

    /** The settled host work the loop has not answered yet. */
    get hostReady(): number {
        return this.#answers.length;
    }

    /** The work settled so far. */
    get settled(): number {
        return this.#settled;
    }

    /** Keep settled host work for the loop to answer in order, or answer it at once. */
    #ordered<Value>(work: Promise<Value>, flight: Flight): Promise<Value> {
        // number the work in the order it began
        const order = this.#sequence;
        this.#sequence += 1;
        const answered = Promise.withResolvers<Value>();
        const fail = (error: unknown) => answered.reject(error);
        const settle = (answer: () => void) => {
            // mark it settled, then refuse the work once the loop failed
            flight.isSettled = true;
            if (this.#failure !== undefined) {
                fail(this.#failure.error);
            }
            // leave it for the loop
            else if (this.#isOrdered) {
                this.#answers.push({ order, by: flight.by, answer, fail });
                this.#wake();
            }
            // answer it at once without a loop
            else {
                answer();
            }
        };
        void work.then(
            (value) => settle(() => answered.resolve(value)),
            (error: unknown) => settle(() => answered.reject(error)),
        );

        return answered.promise;
    }

    /** Count work in flight until it settles either way, waking the waiters. */
    #count<Value>(work: Promise<Value>, flight: Flight): Promise<Value> {
        // count it with where it began, the host's part apart
        const { by, began } = flight;
        this.#flights.add(flight);
        this.#hosts += by === "world" ? 0 : 1;
        this.#recorded?.push(`${by} ${SimulatedScheduler.#site(began)}`);
        this.#wake();
        const end = this.#span(by, began);

        // uncount it once it settles, and wake the waiters
        const settle = (failure?: { readonly error: unknown }) => {
            // uncount it, then wake each waiter
            this.#flights.delete(flight);
            this.#hosts -= by === "world" ? 0 : 1;
            this.#settled += 1;
            end?.(failure);
            this.#wake();
        };
        void work.then(
            () => settle(),
            (error: unknown) => settle({ error }),
        );

        return work;
    }

    /** Open a span for work while tracing, under the active span, returning its end. */
    #span(
        by: Finisher,
        began: Error,
    ): ((failure?: { readonly error: unknown }) => void) | undefined {
        // trace nothing without telemetry
        const telemetry = this.#telemetry;
        if (telemetry === undefined) {
            return undefined;
        }

        // open the span on the world's clock, with who finishes it and where it began
        const { clock, context } = telemetry;
        const span = telemetry.tracers.getTracer(SIMULATION_SCOPE).startSpan(
            `${SIMULATION_SCOPE}.track`,
            {
                startTime: clock.now(),
                attributes: {
                    [`${SIMULATION_SCOPE}.by`]: by,
                    "code.stacktrace": SimulatedScheduler.#site(began),
                },
            },
            context.active(),
        );

        return (failure) => {
            // mark a failure, then end on the world's clock
            if (failure !== undefined) {
                span.setStatus({ code: SpanStatusCode.ERROR, message: String(failure.error) });
            }
            span.end(clock.now());
        };
    }

    /** Wake each waiter for the next activity. */
    #wake(): void {
        for (const resolve of this.#activity.splice(0)) {
            resolve();
        }
    }

    /** Read where work began, as its caller's stack frames. */
    static #site(began: Error): string {
        return (began.stack ?? "").split("\n").slice(2, 8).join(" < ");
    }
}

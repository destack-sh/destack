import { AsyncLocalStorage } from "node:async_hooks";
import { SpanStatusCode } from "@opentelemetry/api";
import { yieldToEventLoop } from "../clock/wait.ts";
import type { Telemetry } from "../telemetry/telemetry.ts";
import type { RunOptions, Scheduler } from "./scheduler.ts";
import type { Simulation } from "../simulation/simulation.ts";

/** The instrumentation scope of a world's spans and log records. */
export const SIMULATION_SCOPE = "destack.simulation";

/** The most real milliseconds jitter delays real work's settling: past a host event loop turn. */
const JITTER_MILLISECONDS = 4;

/** What real or world work is: an ordered operation, a lifetime, or the world's own task. */
export type WorkKind = "operation" | "lifetime" | "task";

/** Work in flight: what it is, where it began, and whether its answer waits for the loop. */
interface Flight {
    /** What it is. */
    readonly kind: WorkKind;
    /** Whether an operation may run long, as a compile or a remote request. */
    readonly isLong: boolean;
    /** Where it began, as an error whose stack is read only when listed. */
    readonly began: Error;
    /** Whether it settled, its answer waiting for the loop to release it. */
    isSettled: boolean;
}

/** Settled real work the loop has not answered yet. */
interface Answer {
    /** The order the work began in among its kind. */
    readonly order: number;
    /** Answer the work with what it settled with. */
    readonly answer: () => void;
    /** Fail the work with the loop's failure. */
    readonly fail: (error: unknown) => void;
}

/** A world's scheduler: operations answered in the order they began, lifetimes at quiescence. */
export class SimulatedScheduler implements Scheduler {
    /** Whether a loop releases settled real work, instead of answering it at once. */
    #isOrdered: boolean;
    /** The work in flight. */
    readonly #flights = new Set<Flight>();
    /** The real work in flight: operations and lifetimes not answered yet. */
    #real = 0;
    /** The operations running, not settled yet, and those of them that run short. */
    #running = { all: 0, short: 0 };
    /** The order the next operation and the next lifetime begin in. */
    readonly #sequence = { operation: 0, lifetime: 0 };
    /** The order of the next operation to answer. */
    #next = 0;
    /** The settled operations the loop has not answered yet, by their order. */
    readonly #operations = new Map<number, Answer>();
    /** The settled lifetimes the loop has not answered yet. */
    readonly #lifetimes: Answer[] = [];
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
    /** The operation the running code started in, absent in the world's own code. */
    readonly #operation = new AsyncLocalStorage<Flight>();
    /** The state of the seeded generator delaying real work's settling, absent without jitter. */
    #jitter: number | undefined;

    /** Schedule a world's work, releasing real work in order when ordered, traced as asked. */
    constructor(options: {
        readonly isOrdered: boolean;
        readonly telemetry?: Telemetry;
        readonly world?: Simulation;
        /** The seed delaying each real work's settling by a real random time, absent for none. */
        readonly jitter?: number;
    }) {
        // keep the options, leaving the failure unhandled until a wait races it
        this.#isOrdered = options.isOrdered;
        this.#jitter = options.jitter;
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

    /** Let the host's event loop run once, as the world's task, as no real work answers it. */
    yield(): Promise<void> {
        return this.task(yieldToEventLoop);
    }

    /** Run real work as one operation, answered in the order operations began. */
    run<Value>(start: () => Promise<Value>, options: RunOptions = {}): Promise<Value> {
        // run real work a running operation begins as part of it, as the world sees the operation whole
        if (this.#runningOperation() !== undefined) {
            return Promise.try(start);
        }

        // start the work inside its flight, so the world work it begins is refused
        const flight = this.#flight("operation", options.isLong === true);
        const work = this.#operation.run(flight, () => Promise.try(start));

        // count it running until it settles, short or long
        const running = this.#running;
        const isShort = !flight.isLong;
        running.all += 1;
        running.short += isShort ? 1 : 0;
        const ran = () => {
            running.all -= 1;
            running.short -= isShort ? 1 : 0;
        };
        void work.then(ran, ran);

        // answer it in its order
        const order = this.#sequence.operation;
        this.#sequence.operation += 1;

        return this.#count(
            this.#held(work, flight, (answer) => this.#operations.set(order, { ...answer, order })),
            flight,
        );
    }

    /** Keep the world's time until real work ends, answering it once the world is quiescent. */
    waitUntil<Value>(settled: Promise<Value>): Promise<Value> {
        // wait for real work a running operation begins as part of it, as the world sees it whole
        if (this.#runningOperation() !== undefined) {
            return settled;
        }

        // answer it among the settled lifetimes, in the order they began
        const flight = this.#flight("lifetime", true);
        const order = this.#sequence.lifetime;
        this.#sequence.lifetime += 1;

        return this.#count(
            this.#held(settled, flight, (answer) => {
                this.#lifetimes.push({ ...answer, order });
                this.#lifetimes.sort((left, right) => left.order - right.order);
            }),
            flight,
        );
    }

    /** Run the world's own work, counted until it settles, refused inside an operation. */
    task<Value>(start: () => Promise<Value>): Promise<Value> {
        // refuse world work an operation begins, as the operation keeps the world's time until it settles
        const flight = this.#flight("task", false);
        const operation = this.#runningOperation();
        if (operation !== undefined) {
            const error = new Error(
                "world work began inside an operation, which keeps the world's time until it settles: wait for the real work with waitUntil",
                {
                    cause: {
                        world: SimulatedScheduler.#site(flight.began),
                        operation: SimulatedScheduler.#site(operation.began),
                    },
                },
            );
            this.fail(error);

            return Promise.reject(error);
        }

        return this.#count(Promise.try(start), flight);
    }

    /** Answer the settled operations whose earlier operations are all answered, in their order. */
    answerOperations(): readonly number[] {
        // answer the settled run of operations from the next one
        const answered: number[] = [];
        for (
            let answer = this.#operations.get(this.#next);
            answer !== undefined;
            answer = this.#operations.get(this.#next)
        ) {
            this.#operations.delete(this.#next);
            answer.answer();
            answered.push(this.#next);
            this.#next += 1;
        }

        return answered;
    }

    /** Answer the settled lifetimes in the order they began, as the world is quiescent. */
    answerLifetimes(): readonly number[] {
        const answers = this.#lifetimes.splice(0);
        for (const each of answers) {
            each.answer();
        }

        return answers.map(({ order }) => order);
    }

    /** Answer real work at once from now on, settled answers first, once no loop orders it. */
    detach(): void {
        this.#isOrdered = false;
        for (const [order, answer] of [...this.#operations].toSorted(
            ([left], [right]) => left - right,
        )) {
            this.#operations.delete(order);
            answer.answer();
        }
        this.answerLifetimes();
    }

    /** Fail the world with its first failure, refusing settled and later answers, counting them. */
    fail(error: unknown): number {
        // keep the first failure, rejecting its waiters and waking the loop to fail the world
        this.#failure ??= { error };
        this.#failed.reject(this.#failure.error);
        this.#wake();

        // fail the settled answers
        const answers = [...this.#operations.values(), ...this.#lifetimes.splice(0)];
        this.#operations.clear();
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

    /** List the work in flight: what it is, whether it runs or settled, and where it began. */
    pending(): string[] {
        return [...this.#flights].map(
            ({ kind, began, isSettled }) =>
                `${kind} ${isSettled ? "settled" : "running"} ${SimulatedScheduler.#site(began)}`,
        );
    }

    /** The work in flight. */
    get inFlight(): number {
        return this.#flights.size;
    }

    /** The real work in flight: operations and lifetimes not answered yet. */
    get real(): number {
        return this.#real;
    }

    /** The operations running, all of them and the short ones. */
    get running(): { readonly all: number; readonly short: number } {
        return { ...this.#running };
    }

    /** Whether the next operation in order settled, so the loop can answer it. */
    get isAnswerable(): boolean {
        return this.#operations.has(this.#next);
    }

    /** The settled lifetimes the loop has not answered yet. */
    get lifetimes(): number {
        return this.#lifetimes.length;
    }

    /** The work settled so far. */
    get settled(): number {
        return this.#settled;
    }

    /** Read the operation the running code is inside, absent once it settled or outside one. */
    #runningOperation(): Flight | undefined {
        const operation = this.#operation.getStore();

        return operation?.isSettled === false ? operation : undefined;
    }

    /** Begin a flight of work, noting where it began. */
    #flight(kind: WorkKind, isLong: boolean): Flight {
        return { kind, isLong, began: new Error(kind), isSettled: false };
    }

    /** Keep settled real work for the loop to release, or answer it at once without a loop. */
    #held<Value>(
        work: Promise<Value>,
        flight: Flight,
        keep: (answer: Omit<Answer, "order">) => void,
    ): Promise<Value> {
        // answer the work once released, or fail it with the loop's failure
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
                keep({ answer, fail });
                this.#wake();
            }
            // answer it at once without a loop
            else {
                answer();
            }
        };
        const delayed = (answer: () => void) => {
            // settle at once, or after a seeded random real delay while jittering
            const delay = this.#delay();
            if (delay === undefined) {
                settle(answer);
            } else {
                setTimeout(() => settle(answer), delay);
            }
        };
        void work.then(
            (value) => delayed(() => answered.resolve(value)),
            (error: unknown) => delayed(() => answered.reject(error)),
        );

        return answered.promise;
    }

    /** Draw the next jitter delay in real milliseconds with mulberry32, absent without jitter. */
    #delay(): number | undefined {
        if (this.#jitter === undefined) {
            return undefined;
        }
        this.#jitter = (this.#jitter + 0x6d2b79f5) | 0;
        let mixed = Math.imul(this.#jitter ^ (this.#jitter >>> 15), 1 | this.#jitter);
        mixed = (mixed + Math.imul(mixed ^ (mixed >>> 7), 61 | mixed)) ^ mixed;
        const unit = ((mixed ^ (mixed >>> 14)) >>> 0) / 4_294_967_296;

        return Math.floor(unit * (JITTER_MILLISECONDS + 1));
    }

    /** Count work in flight until it settles either way, waking the waiters. */
    #count<Value>(work: Promise<Value>, flight: Flight): Promise<Value> {
        // count it with where it began, real work apart
        const { kind, began } = flight;
        const isReal = kind !== "task";
        this.#flights.add(flight);
        this.#real += isReal ? 1 : 0;
        this.#recorded?.push(`${kind} ${SimulatedScheduler.#site(began)}`);
        this.#wake();
        const end = this.#span(kind, began);

        // uncount it once answered, and wake the waiters
        const settle = (failure?: { readonly error: unknown }) => {
            // uncount it, then wake each waiter
            this.#flights.delete(flight);
            this.#real -= isReal ? 1 : 0;
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
        kind: WorkKind,
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
            `${SIMULATION_SCOPE}.work`,
            {
                startTime: clock.now(),
                attributes: {
                    [`${SIMULATION_SCOPE}.kind`]: kind,
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

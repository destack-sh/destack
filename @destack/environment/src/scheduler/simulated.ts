import { AsyncLocalStorage } from "node:async_hooks";
import { SpanStatusCode } from "@opentelemetry/api";
import { Failure } from "@destack/error";
import { yieldToEventLoop } from "../clock/wait.ts";
import { recordSpanException } from "../telemetry/exception.ts";
import type { Telemetry } from "../telemetry/telemetry.ts";
import type { RunOptions, Scheduler } from "./scheduler.ts";
import type { Simulation } from "../simulation/simulation.ts";

/** The instrumentation scope of a world's spans and log records. */
export const SIMULATION_SCOPE = "destack.simulation";

/** The most real milliseconds jitter delays real work's settling: past a host event loop turn. */
const JITTER_MILLISECONDS = 4;

/** What work is: a real operation, short or long, or the world's own task. */
export type WorkKind = "operation" | "task";

/** Work in flight: what it is, where it began, and whether its completion waits for the loop. */
interface Flight {
    /** What it is. */
    readonly kind: WorkKind;
    /** Whether an operation may run long, as a compile or a remote request. */
    readonly isLong: boolean;
    /** Where it began, as an error whose stack is read only when listed. */
    readonly began: Error;
    /** Whether it settled, its completion waiting for the loop. */
    isSettled: boolean;
}

/** A settled operation the loop has not completed yet. */
interface Completion {
    /** The order the operation began in among the short or the long ones. */
    readonly order: number;
    /** Complete the operation with what it settled with. */
    readonly complete: () => void;
    /** Fail the work with the loop's failure. */
    readonly fail: (error: unknown) => void;
}

/** A world's scheduler: short operations complete in settled batches, long ones at idle. */
export class SimulatedScheduler implements Scheduler {
    /** Whether a loop completes settled operations, instead of completing them at once. */
    #isOrdered: boolean;
    /** The work in flight. */
    readonly #flights = new Set<Flight>();
    /** The operations in flight, not completed yet. */
    #real = 0;
    /** The operations running, not settled yet, and those of them that run short. */
    #running = { all: 0, short: 0 };
    /** The order the next short and the next long operation begin in. */
    readonly #sequence = { short: 0, long: 0 };
    /** The order of the next short operation to complete. */
    #next = 0;
    /** The settled short operations the loop has not completed yet, by their order. */
    readonly #short = new Map<number, Completion>();
    /** The settled long operations the loop has not completed yet, in their order. */
    readonly #long: Completion[] = [];
    /** The work settled so far. */
    #settled = 0;
    /** The waiters for the next work to begin or settle. */
    #activity: (() => void)[] = [];
    /** The world's failure, refusing every later completion. */
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

    /** Schedule a world's work, completing operations in order when ordered, traced as asked. */
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

    /** Let the host's event loop run once, as the world's task, as no operation completes it. */
    yield(): Promise<void> {
        return this.task(yieldToEventLoop);
    }

    /** Run real work as one operation: a short one completes in its batch, a long one at idle. */
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

        // complete a long one at idle, as its length follows the host
        if (flight.isLong) {
            return this.#holdLong(work, flight);
        }

        // complete a short one in its order, in the batch settling with it
        const order = this.#sequence.short;
        this.#sequence.short += 1;

        return this.#count(
            this.#held(work, flight, (completion) =>
                this.#short.set(order, { ...completion, order }),
            ),
            flight,
        );
    }

    /** Keep the world's time until real work ends, completing it as a long operation at idle. */
    waitUntil<Value>(settled: Promise<Value>): Promise<Value> {
        // wait for real work a running operation begins as part of it, as the world sees it whole
        if (this.#runningOperation() !== undefined) {
            return settled;
        }

        // complete it as a long operation
        return this.#holdLong(settled, this.#flight("operation", true));
    }

    /** Hold a long operation until the world is idle, completing it in the order it began. */
    #holdLong<Value>(work: Promise<Value>, flight: Flight): Promise<Value> {
        const order = this.#sequence.long;
        this.#sequence.long += 1;

        return this.#count(
            this.#held(work, flight, (completion) => {
                this.#long.push({ ...completion, order });
                this.#long.sort((left, right) => left.order - right.order);
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

    /** Complete the settled short operations in the order they began. */
    completeShort(): readonly number[] {
        // complete the settled run of short operations from the next one
        const completed: number[] = [];
        for (
            let completion = this.#short.get(this.#next);
            completion !== undefined;
            completion = this.#short.get(this.#next)
        ) {
            this.#short.delete(this.#next);
            completion.complete();
            completed.push(this.#next);
            this.#next += 1;
        }

        return completed;
    }

    /** Complete the settled long operations in the order they began, as the world is idle. */
    completeLong(): readonly number[] {
        const completions = this.#long.splice(0);
        for (const each of completions) {
            each.complete();
        }

        return completions.map(({ order }) => order);
    }

    /** Complete operations at once from now on, settled ones first, once no loop orders them. */
    detach(): void {
        this.#isOrdered = false;
        for (const [order, completion] of [...this.#short].toSorted(
            ([left], [right]) => left - right,
        )) {
            this.#short.delete(order);
            completion.complete();
        }
        this.completeLong();
    }

    /** Fail the world with its first failure, returning how many settled completions it refused. */
    fail(error: unknown): number {
        // keep the first failure, rejecting its waiters and waking the loop to fail the world
        this.#failure ??= { error };
        this.#failed.reject(this.#failure.error);
        this.#wake();

        // fail the settled completions
        const completions = [...this.#short.values(), ...this.#long.splice(0)];
        this.#short.clear();
        for (const each of completions) {
            each.fail(this.#failure.error);
        }

        return completions.length;
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

    /** The operations in flight, not completed yet. */
    get real(): number {
        return this.#real;
    }

    /** The operations running, all of them and the short ones. */
    get running(): { readonly all: number; readonly short: number } {
        return { ...this.#running };
    }

    /** Whether every uncompleted short operation settled, to complete them as one batch. */
    get isShortSettled(): boolean {
        const uncompleted = this.#sequence.short - this.#next;

        return uncompleted > 0 && this.#short.size === uncompleted;
    }

    /** Whether every short operation begun so far completed, so long ones may. */
    get isShortCompleted(): boolean {
        return this.#next === this.#sequence.short;
    }

    /** The settled long operations the loop has not completed yet. */
    get settledLong(): number {
        return this.#long.length;
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

    /** Keep a settled operation for the loop to complete, or complete it at once without a loop. */
    #held<Value>(
        work: Promise<Value>,
        flight: Flight,
        keep: (completion: Omit<Completion, "order">) => void,
    ): Promise<Value> {
        // complete the work once the loop does, or fail it with the loop's failure
        const completed = Promise.withResolvers<Value>();
        const fail = (error: unknown) => completed.reject(error);
        const settle = (complete: () => void) => {
            // mark it settled, then refuse the work once the loop failed
            flight.isSettled = true;
            if (this.#failure !== undefined) {
                fail(this.#failure.error);
            }
            // leave it for the loop
            else if (this.#isOrdered) {
                keep({ complete, fail });
                this.#wake();
            }
            // complete it at once without a loop
            else {
                complete();
            }
        };
        const delayed = (complete: () => void) => {
            // settle at once, or after a seeded random real delay while jittering
            const delay = this.#delay();
            if (delay === undefined) {
                settle(complete);
            } else {
                setTimeout(() => settle(complete), delay);
            }
        };
        void work.then(
            (value) => delayed(() => completed.resolve(value)),
            (error: unknown) => delayed(() => completed.reject(error)),
        );

        return completed.promise;
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

        // uncount it once completed, and wake the waiters
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
            // record a failure as the span runner does, then end on the world's clock
            if (failure !== undefined) {
                recordSpanException(span, failure.error);
                span.setStatus({ code: SpanStatusCode.ERROR });
                span.setAttribute("error.type", Failure.of(failure.error).code);
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

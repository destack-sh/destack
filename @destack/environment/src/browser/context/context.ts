import type { Context, ContextManager } from "@opentelemetry/api";

/** The registered symbol a realm's context sits under, as compiled browser modules read it. */
export const CONTEXT_SYMBOL = Symbol.for("destack.context");

/** A context's values by key, each change returning a new context. */
class ContextValues implements Context {
    /** The values by key. */
    readonly #values: ReadonlyMap<symbol, unknown>;

    /** Hold values by key. */
    constructor(values: ReadonlyMap<symbol, unknown>) {
        this.#values = values;
    }

    /** Read a key's value. */
    getValue(key: symbol): unknown {
        return this.#values.get(key);
    }

    /** Return a context with a key set to a value. */
    setValue(key: symbol, value: unknown): Context {
        return new ContextValues(new Map(this.#values).set(key, value));
    }

    /** Return a context without a key. */
    deleteValue(key: symbol): Context {
        const values = new Map(this.#values);
        values.delete(key);

        return new ContextValues(values);
    }
}

/** The root context, holding no values, defined here as the baseline loads before OpenTelemetry. */
const ROOT_CONTEXT: Context = new ContextValues(new Map());

/** The global object holding a realm's context. */
type ContextGlobal = typeof globalThis & { [CONTEXT_SYMBOL]?: BrowserContext };

/**
 * The telemetry context active in a browser realm, carried across awaits by compiled modules.
 *
 * Browsers keep no async context, so the build opens a frame in each async function.
 * The frame restores the function's context at each resume and resets the realm's at each suspend.
 */
export class BrowserContext implements ContextManager {
    /** The context active now. */
    #active: Context = ROOT_CONTEXT;

    /** Install the realm's context once, returning the installed one. */
    static install(): BrowserContext {
        const realm: ContextGlobal = globalThis;
        realm[CONTEXT_SYMBOL] ??= new BrowserContext();

        return realm[CONTEXT_SYMBOL];
    }

    /** Read the context active now. */
    active(): Context {
        return this.#active;
    }

    /** Run a callback in a context, restoring the previous one once it returns. */
    with<
        Arguments extends unknown[],
        Callback extends (...given: Arguments) => ReturnType<Callback>,
    >(
        context: Context,
        callback: Callback,
        receiver?: ThisParameterType<Callback>,
        ...given: Arguments
    ): ReturnType<Callback> {
        // enter the context for the callback's synchronous run
        const previous = this.#active;
        this.#active = context;
        try {
            return callback.apply(receiver, given);
        } finally {
            this.#active = previous;
        }
    }

    /** Bind a function to run in a context, leaving other targets as they are. */
    bind<Target>(context: Context, target: Target): Target {
        // run each call of a function in the context it was bound in
        if (typeof target === "function") {
            return new Proxy(target, {
                apply: (bound, receiver, given: unknown[]): unknown =>
                    this.with(context, () => {
                        const value: unknown = Reflect.apply(bound, receiver, given);

                        return value;
                    }),
            });
        }

        return target;
    }

    /** Enable the manager, which is always enabled. */
    enable(): this {
        return this;
    }

    /** Disable the manager, resetting the active context. */
    disable(): this {
        this.#active = ROOT_CONTEXT;

        return this;
    }

    /** Open a frame for an async function's call, capturing the context active at its start. */
    enter(): ContextFrame {
        return new ContextFrame(this, this.#active);
    }

    /** Set the active context, as a frame resumes or suspends. */
    activate(context: Context): void {
        this.#active = context;
    }
}

/** One async function call's context: restored at each resume, reset at each suspend. */
export class ContextFrame {
    /** The realm's context. */
    readonly #realm: BrowserContext;
    /** The context active when the call started. */
    readonly #context: Context;
    /** Whether the call suspended once, so each later segment runs from the event loop. */
    #isSuspended = false;

    /** Capture a call's starting context in a realm. */
    constructor(realm: BrowserContext, context: Context) {
        this.#realm = realm;
        this.#context = context;
    }

    /** Suspend at an await or yield, handing the value back. */
    suspend<Value>(value: Value): Value {
        // hand a later segment's end to the event loop under the root, the first one's back to the caller
        if (this.#isSuspended) {
            this.#realm.activate(ROOT_CONTEXT);
        }
        this.#isSuspended = true;

        return value;
    }

    /** Resume after an await or yield, or in a catch, finally or loop body. */
    resume<Value>(value?: Value): Value | undefined {
        // restore the call's context once it left it
        if (this.#isSuspended) {
            this.#realm.activate(this.#context);
        }

        return value;
    }

    /** Suspend before each step of an async iteration. */
    each<Value>(iterable: AsyncIterable<Value> | Iterable<Value>): AsyncIterable<Value> {
        // suspend before each step the loop awaits, resuming in the loop's body
        return {
            [Symbol.asyncIterator]: () => {
                const iterator =
                    Symbol.asyncIterator in iterable
                        ? iterable[Symbol.asyncIterator]()
                        : iterableAsync(iterable);

                return {
                    next: (...given: [] | [unknown]) => this.suspend(iterator.next(...given)),
                    return: (value?: unknown) =>
                        iterator.return === undefined
                            ? Promise.resolve({ done: true as const, value: undefined })
                            : iterator.return(value),
                };
            },
        };
    }

    /** Exit the call, handing the event loop the root context once it suspended. */
    exit(): void {
        // reset the realm's context once the call ends from the event loop
        if (this.#isSuspended) {
            this.#realm.activate(ROOT_CONTEXT);
        }
    }
}

/** Iterate a synchronous iterable as a for await loop does, without async functions. */
function iterableAsync<Value>(iterable: Iterable<Value>): AsyncIterator<Value> {
    // await each value the synchronous iterator steps to, as this module runs before the build's frames exist
    const iterator = iterable[Symbol.iterator]();

    return {
        next: () => {
            const step = iterator.next();

            return step.done === true
                ? Promise.resolve(step)
                : Promise.resolve(step.value).then((value) => ({ done: false, value }));
        },
        return: (value?: unknown) => {
            iterator.return?.(value);

            return Promise.resolve({ done: true, value: undefined });
        },
    };
}

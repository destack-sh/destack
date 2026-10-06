import { AsyncLocalStorage } from "node:async_hooks";
import { type Context, type ContextManager, ROOT_CONTEXT } from "@opentelemetry/api";

/** Keep the active context in the AsyncLocalStorage workerd provides under nodejs_compat. */
export class WorkerContextManager implements ContextManager {
    /** The context of each asynchronous flow. */
    readonly #storage = new AsyncLocalStorage<Context>();
    /** Whether the manager keeps contexts, between its enabling and its disabling. */
    #isEnabled = true;

    /** Read the active context, the root outside every flow and once disabled. */
    active(): Context {
        return this.#isEnabled ? (this.#storage.getStore() ?? ROOT_CONTEXT) : ROOT_CONTEXT;
    }

    /** Run a function with a context active for its whole flow. */
    with<
        Arguments extends unknown[],
        Callback extends (...values: Arguments) => ReturnType<Callback>,
    >(
        context: Context,
        callback: Callback,
        receiver?: ThisParameterType<Callback>,
        ...values: Arguments
    ): ReturnType<Callback> {
        return this.#storage.run(context, () => callback.apply(receiver, values));
    }

    /** Bind a function to a context, leaving other values as they are. */
    bind<Target>(context: Context, target: Target): Target {
        // leave values that are no functions as they are
        if (typeof target !== "function") {
            return target;
        }

        return new Proxy(target, {
            apply: (called, receiver: unknown, values: unknown[]): unknown =>
                this.#storage.run(context, (): unknown => Reflect.apply(called, receiver, values)),
        });
    }

    /** Start keeping contexts. */
    enable(): this {
        this.#isEnabled = true;

        return this;
    }

    /** Stop keeping contexts, as workerd's AsyncLocalStorage implements no disabling. */
    disable(): this {
        this.#isEnabled = false;

        return this;
    }
}

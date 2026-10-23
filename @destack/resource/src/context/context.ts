import { ResourceError } from "../error/index.ts";
import type { ResourceHandle } from "../declare/handle.ts";

/** Resource clients authorised by the host for one invocation or local operation. */
export class ResourceContext {
    /** Clients indexed by their imported declaration objects. */
    readonly #clients = new WeakMap<object, unknown>();

    /** Bind an authorised client before invoking application code. */
    bind<Value>(resource: ResourceHandle<Value>, client: NoInfer<Value>): this {
        // bind each declaration once
        if (this.#clients.has(resource)) {
            throw ResourceError.of("ALREADY_BOUND", `resource already bound: ${resource.name}`);
        }

        // record the client
        this.#clients.set(resource, client);

        return this;
    }

    /** Return the client selected for this declaration. */
    get<Value>(resource: ResourceHandle<Value>): Value;
    /**
     * Return the client bound to the declaration, which bind keeps under its own declaration.
     *
     * NOTE #Architecture: the one stated type of resource clients, as React's typed context.
     */
    get(resource: ResourceHandle<unknown>): unknown {
        // require a bound client
        if (!this.#clients.has(resource)) {
            throw ResourceError.of("NOT_BOUND", `resource is not bound: ${resource.name}`);
        }

        return this.#clients.get(resource);
    }
}

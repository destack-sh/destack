import { ResourceError } from "../error/index.ts";
import type { Resource } from "../resource/handle.ts";

/** Resource clients authorised by the host for one invocation or local operation. */
export class ResourceContext {
    /** Clients indexed by their imported declaration objects. */
    readonly #clients = new WeakMap<object, unknown>();

    /** Bind an authorised client before invoking application code. */
    bind<Value>(resource: Resource<Value>, client: NoInfer<Value>): this {
        if (this.#clients.has(resource)) {
            throw new ResourceError("ALREADY_BOUND", `Resource already bound: ${resource.name}`);
        }
        this.#clients.set(resource, client);

        return this;
    }

    /** Return the client selected for this declaration. */
    get<Value>(resource: Resource<Value>): Value {
        if (!this.#clients.has(resource)) {
            throw new ResourceError("NOT_BOUND", `Resource is not bound: ${resource.name}`);
        }

        return this.#clients.get(resource) as Value;
    }
}

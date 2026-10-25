import { WorkerdEnvironment } from "@destack/environment/workerd";
import type { DurableObjectStorage } from "./client.ts";
import {
    DurableObjectDatabaseHost,
    type DurableObjectDatabaseBindings,
    type DurableObjectNamespace,
    Jurisdiction,
} from "./host.ts";

/** A Durable Object answering operations on its databases. */
export class DurableObjectDatabase {
    /** The object's databases. */
    readonly host: DurableObjectDatabaseHost;
    /** The partitions' objects, in their jurisdiction. */
    readonly #partitions: DurableObjectNamespace;

    /** Store databases in the object's storage, with partition objects from Worker bindings. */
    constructor(
        state: { readonly storage: DurableObjectStorage },
        bindings: DurableObjectDatabaseBindings,
    ) {
        this.host = new DurableObjectDatabaseHost(state.storage, WorkerdEnvironment.system);
        this.#partitions = Jurisdiction.narrow(bindings.PARTITION, bindings.DESTACK_JURISDICTION);
    }

    /** Answer an operation on a stored database, copying through its partition's object. */
    fetch(request: Request): Promise<Response> {
        return this.host.answer(request, (scope) =>
            this.#partitions.get(this.#partitions.idFromName(scope)),
        );
    }
}

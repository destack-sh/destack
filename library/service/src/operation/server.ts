import { implement } from "../server/handler.ts";
import { defineOperationProcedures } from "./procedure.ts";
import type { OperationStore } from "./store.ts";

/** Verified authorization state supplied by the host for every operation request. */
export interface OperationRequest {
    /** An authenticated scope; never copied directly from a request parameter. */
    owner: string;
}

/** Implement the shared operation API using an authorized in-memory store. */
export function implementOperation<Result, Progress>(
    store: OperationStore<Result, Progress>,
    path: `/${string}` = "/operations",
) {
    // implement the typed procedures under the host's authenticated context
    const definition = defineOperationProcedures(store.definition, path);
    const implementation = implement(definition).$context<OperationRequest>();

    return implementation.router({
        get: implementation.get.handler(({ input, context }) => store.get(context.owner, input.id)),
        list: implementation.list.handler(({ context }) => store.list(context.owner)),
        watch: implementation.watch.handler(({ input, context, signal }) =>
            store.watch(context.owner, input.id, signal),
        ),
        cancel: implementation.cancel.handler(({ input, context }) =>
            store.cancel(context.owner, input.id),
        ),
        delete: implementation.delete.handler(({ input, context }) => {
            store.delete(context.owner, input.id);

            return null;
        }),
    });
}

import type { ServiceContext } from "../server/context.ts";
import { implement } from "../server/handler.ts";
import { defineOperationProcedures } from "./procedure.ts";
import type { OperationStore } from "./store.ts";

/** Implement the shared operation API using an authorized in-memory store. */
export function implementOperation<Result, Progress>(
    store: OperationStore<Result, Progress>,
    path: `/${string}` = "/operations",
) {
    // implement the typed procedures under the host's authenticated context
    const definition = defineOperationProcedures(store.definition, path);
    const implementation = implement(definition).$context<ServiceContext>();

    return implementation.router({
        get: implementation.get.handler(({ input, context }) =>
            store.get(context.requireCaller().id, input.id),
        ),
        list: implementation.list.handler(({ context }) => store.list(context.requireCaller().id)),
        watch: implementation.watch.handler(({ input, context, signal }) =>
            store.watch(context.requireCaller().id, input.id, signal),
        ),
        cancel: implementation.cancel.handler(({ input, context }) =>
            store.cancel(context.requireCaller().id, input.id),
        ),
        delete: implementation.delete.handler(({ input, context }) => {
            store.delete(context.requireCaller().id, input.id);

            return null;
        }),
    });
}

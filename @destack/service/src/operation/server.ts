import type { ServiceContext } from "../server/context.ts";
import { implement } from "../server/handler.ts";
import { defineOperationProcedures } from "./procedure.ts";
import type { OperationStore } from "./store.ts";

/** Implement the operation procedures on a store. */
export function implementOperation<Result, Progress>(
    store: OperationStore<Result, Progress>,
    path: `/${string}` = "/operations",
) {
    // implement the procedures
    const definition = defineOperationProcedures(store.definition, path);
    const implementation = implement(definition).$context<ServiceContext>();

    return implementation.router({
        get: implementation.get.handler(({ input, context }) =>
            store.get(context.requireAuthentication().id, input.id),
        ),
        list: implementation.list.handler(({ context }) =>
            store.list(context.requireAuthentication().id),
        ),
        watch: implementation.watch.handler(({ input, context, signal }) =>
            store.watch(context.requireAuthentication().id, input.id, signal),
        ),
        cancel: implementation.cancel.handler(({ input, context }) =>
            store.cancel(context.requireAuthentication().id, input.id),
        ),
        delete: implementation.delete.handler(({ input, context }) => {
            store.delete(context.requireAuthentication().id, input.id);

            return null;
        }),
    });
}

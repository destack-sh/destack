import { implement, type ServiceContext } from "@destack/service/server";
import { checkout } from "../service/checkout.ts";
import * as domain from "../checkout/index.ts";
import type { AuditRecorder } from "@destack/audit";
import type { DatabaseConnection } from "@destack/db";
import { ServiceError } from "@destack/service";

/** Checkout procedures receive authenticated host state and an audit recorder. */
const implementation = implement(checkout).$context<ServiceContext>();

/** Serve persistent checkout registrations without modifying repository contents. */
export function implementCheckout(
    storage: domain.CheckoutStore,
    record: (context: ServiceContext) => AuditRecorder<DatabaseConnection>,
) {
    return implementation.router({
        create: implementation.create.handler(() => {
            // TODO #Incomplete: initialize Git, register the directory and retain operation progress
            throw new ServiceError("NOT_IMPLEMENTED");
        }),
        clone: implementation.clone.handler(() => {
            // TODO #Incomplete: clone with host credentials and cancellation, then register the result
            throw new ServiceError("NOT_IMPLEMENTED");
        }),
        operation: {
            get: implementation.operation.get.handler(() => {
                // TODO #Incomplete: read retained checkout operation status
                throw new ServiceError("NOT_IMPLEMENTED");
            }),
            watch: implementation.operation.watch.handler(() => {
                // TODO #Incomplete: observe checkout operation progress through cancellation
                throw new ServiceError("NOT_IMPLEMENTED");
            }),
            cancel: implementation.operation.cancel.handler(() => {
                // TODO #Incomplete: stop Git and preserve preexisting user files
                throw new ServiceError("NOT_IMPLEMENTED");
            }),
        },
        relocate: implementation.relocate.handler(() => {
            // TODO #Incomplete: verify the new Git root and update the registered path atomically
            throw new ServiceError("NOT_IMPLEMENTED");
        }),
        status: implementation.status.handler(() => {
            // TODO #Incomplete: read worktree, index, refs and interrupted Git operations
            throw new ServiceError("NOT_IMPLEMENTED");
        }),
        watch: implementation.watch.handler(() => {
            // TODO #Incomplete: reread Git state after debounced directory and metadata invalidations
            throw new ServiceError("NOT_IMPLEMENTED");
        }),
        list: implementation.list.handler(({ input }) => storage.list(input)),
        get: implementation.get.handler(({ input }) => storage.get(input.checkoutId)),
        register: implementation.register.handler(({ input, context }) =>
            domain.registerCheckout(input, storage, record(context)),
        ),
        unregister: implementation.unregister.handler(async ({ input, context }) => {
            await storage.unregister(input.checkoutId, record(context));

            return {};
        }),
    });
}

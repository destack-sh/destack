import { implement } from "@destack/service/server";
import { ServiceError } from "@destack/service";
import { deployment } from "../service/deployment.ts";

/** Typed execution reconciliation procedures. */
const implementation = implement(deployment);

/** Serve accepted execution instructions independently of running instances. */
export const deploymentRouter = implementation.router({
    list: implementation.list.handler(() => {
        // TODO #Incomplete: list durable accepted instructions with stable pagination
        throw new ServiceError("NOT_IMPLEMENTED");
    }),
    get: implementation.get.handler(() => {
        // TODO #Incomplete: read the accepted generation and authorization lifetime
        throw new ServiceError("NOT_IMPLEMENTED");
    }),
    watch: implementation.watch.handler(() => {
        // TODO #Incomplete: observe accepted instructions and expiry through host recovery
        throw new ServiceError("NOT_IMPLEMENTED");
    }),
    apply: implementation.apply.handler(() => {
        // TODO #Incomplete: verify regional instructions, persist them and reconcile bounded restarts
        throw new ServiceError("NOT_IMPLEMENTED");
    }),
});

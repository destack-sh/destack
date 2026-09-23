import { implement } from "@destack/service/server";
import { ServiceError } from "@destack/service/error";
import { instance } from "../service/instance.ts";

/** Typed implementations of the workload procedures. */
const implementation = implement(instance);

/** Serve workload administration requests. */
export const instanceRouter = implementation.router({
    /** Replace a running or failed execution without reusing its instance identity. */
    restart: implementation.restart.handler(() => {
        // TODO #Incomplete: drain the old execution, recheck authority and start a new instance
        throw new ServiceError("NOT_IMPLEMENTED");
    }),
    /** Implement instance.list. */
    list: implementation.list.handler(() => {
        // TODO #Incomplete: list persisted executions with stable pagination
        throw new ServiceError("NOT_IMPLEMENTED");
    }),
    /** Implement instance.get. */
    get: implementation.get.handler(() => {
        // TODO #Incomplete: read persisted execution state and its last observation
        throw new ServiceError("NOT_IMPLEMENTED");
    }),
    /** Implement instance.start. */
    start: implementation.start.handler(() => {
        // TODO #Incomplete: verify deployment and host epoch, prepare resources and launch through the sandbox
        throw new ServiceError("NOT_IMPLEMENTED");
    }),
    /** Implement instance.stop. */
    stop: implementation.stop.handler(() => {
        // TODO #Incomplete: stop this lifetime; active deployment instructions may request a replacement
        throw new ServiceError("NOT_IMPLEMENTED");
    }),
    /** Implement instance.watch. */
    watch: implementation.watch.handler(() => {
        // TODO #Incomplete: observe complete instance state until cancellation
        throw new ServiceError("NOT_IMPLEMENTED");
    }),
});

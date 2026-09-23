import { implement } from "@destack/service/server";
import { ServiceError } from "@destack/service/error";
import { preview } from "../service/preview.ts";

/** Typed implementations of the preview procedures. */
const implementation = implement(preview);

/** Serve preview administration requests. */
export const previewRouter = implementation.router({
    view: {
        list: implementation.view.list.handler(() => {
            // TODO #Incomplete: read the current package views and their BuildService preview references
            throw new ServiceError("NOT_IMPLEMENTED");
        }),
        open: implementation.view.open.handler(() => {
            // TODO #Incomplete: authorize the selected identity and issue a single-use view location
            throw new ServiceError("NOT_IMPLEMENTED");
        }),
    },
    /** Implement preview.list. */
    list: implementation.list.handler(() => {
        // TODO #Incomplete: list retained previews with stable pagination
        throw new ServiceError("NOT_IMPLEMENTED");
    }),
    /** Implement preview.get. */
    get: implementation.get.handler(() => {
        // TODO #Incomplete: read preview selection and current build status
        throw new ServiceError("NOT_IMPLEMENTED");
    }),
    /** Implement preview.start. */
    start: implementation.start.handler(() => {
        // TODO #Incomplete: verify login and regional development space, then start builds and authorized workloads
        throw new ServiceError("NOT_IMPLEMENTED");
    }),
    /** Implement preview.stop. */
    stop: implementation.stop.handler(() => {
        // TODO #Incomplete: close the build watcher and preview executions without changing source files
        throw new ServiceError("NOT_IMPLEMENTED");
    }),
    /** Implement preview.watch. */
    watch: implementation.watch.handler(() => {
        // TODO #Incomplete: observe build and runtime changes until cancellation
        throw new ServiceError("NOT_IMPLEMENTED");
    }),
});

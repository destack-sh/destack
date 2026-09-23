import { implement } from "@destack/service/server";
import { ServiceError } from "@destack/service";
import { host } from "../service/host.ts";

/** Typed host enrollment and execution administration. */
const implementation = implement(host);

/** Serve universe enrollment and persistent execution availability. */
export const hostRouter = implementation.router({
    enrollment: {
        get: implementation.enrollment.get.handler(() => {
            // TODO #Incomplete: read verified enrollment metadata without accessing credentials
            throw new ServiceError("NOT_IMPLEMENTED");
        }),
        enroll: implementation.enrollment.enroll.handler(() => {
            // TODO #Incomplete: exchange the issuer credential and retain host keys in secure OS storage
            throw new ServiceError("NOT_IMPLEMENTED");
        }),
        revoke: implementation.enrollment.revoke.handler(() => {
            // TODO #Incomplete: drain executions, request remote revocation and remove local keys
            throw new ServiceError("NOT_IMPLEMENTED");
        }),
    },
    execution: {
        get: implementation.execution.get.handler(() => {
            // TODO #Incomplete: describe runtime adapters, sandbox support and admission status
            throw new ServiceError("NOT_IMPLEMENTED");
        }),
        watch: implementation.execution.watch.handler(() => {
            // TODO #Incomplete: observe admission and capability changes
            throw new ServiceError("NOT_IMPLEMENTED");
        }),
        drain: implementation.execution.drain.handler(() => {
            // TODO #Incomplete: persist draining status before stopping accepted executions
            throw new ServiceError("NOT_IMPLEMENTED");
        }),
        resume: implementation.execution.resume.handler(() => {
            // TODO #Incomplete: revalidate enrollment and execution instructions before accepting work
            throw new ServiceError("NOT_IMPLEMENTED");
        }),
    },
});

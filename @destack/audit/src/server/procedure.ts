import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service";
import type { ProcedureAudit, ProcedureCall } from "@destack/service/server";
import { defineAuditAction } from "../action/index.ts";
import { AuditRecorder } from "../record/index.ts";
import type { AuditEvent, AuditResult } from "../event/index.ts";
import { AuditError } from "../error/index.ts";
import manifest from "../../package.json" with { type: "json" };
import definition from "../../destack.json" with { type: "json" };
import { PackageId } from "@destack/package";

/** Procedure execution, separate from any domain action committed by its handler. */
export const invokeService = defineAuditAction({
    package: { id: PackageId.parse(definition.id), name: manifest.name, version: manifest.version },
    name: "service.invoke",
    version: 1,
    targets: schema.object({
        procedure: schema.object({ type: schema.literal("procedure"), id: schema.string().min(1) }),
    }),
    details: schema.object({ authentication: schema.enum(["public", "identity", "host"]) }),
});

/** Record service invocations under host-attested request context. */
export function createProcedureAudit<State extends object>(
    recorder: (
        call: ProcedureCall<State>,
    ) =>
        | Pick<AuditRecorder, "begin" | "complete" | "append">
        | Promise<Pick<AuditRecorder, "begin" | "complete" | "append">>,
): (event: ProcedureAudit<State>) => Promise<void> {
    const attempts = new WeakMap<
        ProcedureCall<State>,
        { recorder: Pick<AuditRecorder, "begin" | "complete" | "append">; event: AuditEvent }
    >();

    return async (event) => {
        if (event.outcome === "started") {
            const writer = await recorder(event.call);
            const attempt = writer.begin(invokeService, {
                targets: { procedure: { type: "procedure", id: event.call.path.join(".") } },
                details: { authentication: event.call.access.authentication },
            });
            await writer.append(attempt);
            attempts.set(event.call, { recorder: writer, event: attempt });
        } else {
            // record codes only; exception messages can contain credentials or application contents
            const attempt = attempts.get(event.call);
            if (!attempt) {
                throw new AuditError(
                    "INVALID_EVENT",
                    "audit completion has no recorded procedure attempt",
                );
            }
            const errorCode =
                event.error instanceof ServiceError
                    ? event.error.code
                    : event.outcome === "cancelled"
                      ? "CANCELLED"
                      : "INTERNAL_SERVER_ERROR";
            const result: AuditResult =
                event.outcome === "succeeded"
                    ? { outcome: "success" }
                    : {
                          outcome: event.outcome === "failed" ? "failure" : event.outcome,
                          errorCode,
                      };
            await attempt.recorder.append(attempt.recorder.complete(attempt.event, result));
            attempts.delete(event.call);
        }
    };
}

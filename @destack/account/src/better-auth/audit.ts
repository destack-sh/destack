import { AuditCaller, defineAuditAction } from "@destack/object";
import { AuditRecorder, type Journal } from "@destack/object/server";
import { Scope } from "@destack/sync";
import { present, schema } from "@destack/schema";
import {
    SpanKind,
    SpanStatusCode,
    SeverityNumber,
    telemetry,
    context,
    extractContext,
    isSpanContextValid,
    type Span,
} from "@destack/telemetry";
import type { BetterAuthPlugin } from "better-auth";
import { createAuthMiddleware } from "better-auth/api";
import { principal } from "@destack/access";
import type { Fetch } from "@destack/runtime";
import { sessionCreate } from "../object/session.ts";
import { Package } from "@destack/package";
import accountPackage from "../package.ts";

/** The authentication reads, which leave credentials and challenges unchanged. */
const READS = new Set([
    "/get-session",
    "/list-accounts",
    "/passkey/list-user-passkeys",
    "/oauth2/userinfo",
    "/oauth2/public-client",
    "/ok",
]);

/** Authentication log severity in the shared telemetry model. */
const SEVERITIES = {
    debug: SeverityNumber.DEBUG,
    info: SeverityNumber.INFO,
    warn: SeverityNumber.WARN,
    error: SeverityNumber.ERROR,
};

/** One authentication protocol request with its HTTP method. */
export const authenticationRequest = defineAuditAction({
    name: "authentication.request",
    target: schema.object({ type: schema.literal("authentication-endpoint"), id: schema.string() }),
    details: schema.object({ method: schema.string() }),
});

/** Record Better Auth's requests and the sessions it issues in the account database's journal. */
export class BetterAuthAudit {
    /** The journal of the account database's calls. */
    readonly journal: Journal;
    /** The path Better Auth answers below, the issuer's. */
    readonly basePath: string;

    /** Record authentication calls below a path in the account database's journal. */
    constructor(journal: Journal, basePath: string) {
        this.journal = journal;
        this.basePath = basePath;
    }

    /** Resolve the recorded caller of a request. */
    static caller(current: { user: { id: string } } | null): AuditCaller {
        return current === null
            ? AuditCaller.anyone
            : {
                  subject: principal.user.reference(
                      Scope.universe.id,
                      schema.identifier("user").parse(current.user.id),
                  ),
              };
    }

    /** Report protocol diagnostics with fixed messages and standard severity. */
    log(level: keyof typeof SEVERITIES): void {
        telemetry.scope(Package.of(accountPackage)).logger.emit({
            severityNumber: SEVERITIES[level],
            severityText: level.toUpperCase(),
            body: "authentication protocol diagnostic",
        });
    }

    /** Record issued sessions after authentication plugins have completed their challenges. */
    plugin(): BetterAuthPlugin {
        return {
            id: "destack-audit",
            hooks: {
                after: [
                    {
                        matcher: (endpoint) =>
                            endpoint.context.newSession !== null &&
                            endpoint.context.newSession !== undefined,
                        handler: createAuthMiddleware(async (endpoint) => {
                            // record the sign-in for the new session
                            const current = present(
                                endpoint.context.newSession,
                                "the session a sign-in created",
                            );
                            const userId = schema.identifier("user").parse(current.user.id);
                            const sessionId = schema
                                .identifier("session")
                                .parse(current.session.id);
                            const recorder = new AuditRecorder(
                                {
                                    caller: BetterAuthAudit.caller(current),
                                    package: Package.of(accountPackage),
                                    scope: userId,
                                    sessionId,
                                },
                                this.journal,
                            );

                            // record the issued session's verified identity
                            await this.journal.database.transaction((transaction) =>
                                recorder.record(transaction, sessionCreate, {
                                    target: { type: "session", id: sessionId },
                                    details:
                                        endpoint.path === undefined ? {} : { path: endpoint.path },
                                    outcome: { kind: "success" },
                                }),
                            );
                        }),
                    },
                ],
            },
        };
    }

    /** Trace a request and persist required audit attempts and outcomes. */
    async invoke(
        request: Request,
        caller: AuditCaller,
        routes: readonly string[],
        handler: Fetch,
    ): Promise<Response> {
        // match the request path against the audited routes
        const path = new URL(request.url).pathname.slice(this.basePath.length).split("/");
        const route = routes.find((candidate) => {
            const segments = candidate.split("/");

            return (
                segments.length === path.length &&
                segments.every(
                    (segment, index) => segment.startsWith(":") || segment === path[index],
                )
            );
        });
        if (route === undefined) {
            return handler(request);
        }

        // propagate trace context without credentials
        return context.with(extractContext(request.headers), () =>
            telemetry.scope(Package.of(accountPackage)).tracer.startActiveSpan(
                `authentication ${route}`,
                {
                    kind: SpanKind.SERVER,
                    attributes: {
                        "http.route": `${this.basePath}${route}`,
                        "http.request.method": request.method,
                    },
                },
                async (span) => {
                    try {
                        return await this.#record(request, route, caller, handler, span);
                    } finally {
                        span.end();
                    }
                },
            ),
        );
    }

    /** Persist a request attempt before dispatch and its outcome before returning. */
    async #record(
        request: Request,
        route: string,
        caller: AuditCaller,
        handler: Fetch,
        span: Span,
    ): Promise<Response> {
        // associate durable records with the verified caller and active trace
        const trace = span.spanContext();
        const recorder = new AuditRecorder(
            {
                caller,
                package: Package.of(accountPackage),
                scope: principal.user.is(caller.subject) ? caller.subject.id : Scope.universe.id,
                ...(isSpanContextValid(trace) ? { traceId: trace.traceId } : {}),
            },
            this.journal,
        );
        const attempt = READS.has(route)
            ? undefined
            : recorder.begin(authenticationRequest, {
                  target: { type: "authentication-endpoint", id: route },
                  details: { method: request.method },
              });

        // require durable acceptance before running a credential mutation
        if (attempt !== undefined) {
            await recorder.append(attempt);
        }

        // run the request and record a failure
        const response = await BetterAuthAudit.#dispatch(request, handler, recorder, attempt, span);

        // mark a rejected response on the span
        const isRejected = BetterAuthAudit.#isRejected(request, response);
        span.setAttribute("http.response.status_code", response.status);
        if (isRejected) {
            span.setStatus({ code: SpanStatusCode.ERROR });
        }

        // record the outcome before answering
        if (attempt !== undefined) {
            const outcome = isRejected
                ? {
                      kind: "failure" as const,
                      error: {
                          code: "AUTHENTICATION_REJECTED",
                          status: response.status,
                          message: "authentication rejected",
                      },
                  }
                : { kind: "success" as const };
            await recorder.append(recorder.finish(attempt, outcome));
        }

        return response;
    }

    /** Run a request, and record a failing run of a recorded attempt. */
    static async #dispatch(
        request: Request,
        handler: Fetch,
        recorder: AuditRecorder,
        attempt: ReturnType<AuditRecorder["begin"]> | undefined,
        span: Span,
    ): Promise<Response> {
        try {
            return await handler(request);
        } catch (error) {
            // record the failure of a recorded attempt
            span.setStatus({ code: SpanStatusCode.ERROR });
            if (attempt !== undefined) {
                try {
                    await recorder.append(
                        recorder.finish(attempt, {
                            kind: "failure",
                            error: {
                                code: "INTERNAL_SERVER_ERROR",
                                status: 500,
                                message: "internal error",
                            },
                        }),
                    );
                } catch (auditError) {
                    throw new AggregateError(
                        [error, auditError],
                        "authentication and audit recording failed",
                        { cause: auditError },
                    );
                }
            }

            throw error;
        }
    }

    /** Decide whether a response refuses the request. */
    static #isRejected(request: Request, response: Response): boolean {
        const location = response.headers.get("location");

        return (
            response.status >= 400 ||
            (location !== null && new URL(location, request.url).searchParams.has("error"))
        );
    }
}

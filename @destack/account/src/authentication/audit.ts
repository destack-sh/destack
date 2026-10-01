import { AuditCaller, AuditRecorder, defineAuditAction, type Journal } from "@destack/audit";
import { Scope } from "@destack/sync";
import { identifier, schema } from "@destack/schema";
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
import { accountPackage } from "../audit/record.ts";

/** Authentication reads that do not change credentials or issue challenges. */
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

/** One authentication protocol request, without credentials or provider payloads. */
export const authenticationRequest = defineAuditAction({
    name: "authentication.request",
    targets: schema.object({
        endpoint: schema.object({
            type: schema.literal("authentication-endpoint"),
            id: schema.string(),
        }),
    }),
    details: schema.object({ method: schema.string() }),
});

/** A session issued after all required authentication ceremonies complete. */
export const sessionCreated = defineAuditAction({
    name: "session.create",
    targets: schema.object({
        user: schema.object({ type: schema.literal("user"), id: identifier("user") }),
        session: schema.object({ type: schema.literal("session"), id: identifier("session") }),
    }),
    details: schema.object({
        /** The path of the endpoint issuing the session, absent for a server-only endpoint without one. */
        method: schema.string().optional(),
    }),
});

/** Record authentication HTTP outcomes in the account database's journal. */
export class AuthenticationAudit {
    /** The journal of the account database's calls. */
    readonly journal: Journal;

    /** Record authentication calls in the account database's journal. */
    constructor(journal: Journal) {
        this.journal = journal;
    }

    /** Resolve the recorded caller of a request. */
    static caller(current: { user: { id: string } } | null): AuditCaller {
        return current
            ? {
                  type: "subject",
                  subject: principal.user.reference(
                      Scope.universe.id,
                      identifier("user").parse(current.user.id),
                  ),
              }
            : { type: "anonymous" };
    }

    /** Report protocol diagnostics with fixed messages and standard severity. */
    log(level: keyof typeof SEVERITIES): void {
        telemetry.scope(accountPackage).logger.emit({
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
                        matcher: (context) =>
                            context.context.newSession !== null &&
                            context.context.newSession !== undefined,
                        handler: createAuthMiddleware(async (context) => {
                            // record the sign-in for the new session
                            const current = context.context.newSession!;
                            const userId = identifier("user").parse(current.user.id);
                            const sessionId = identifier("session").parse(current.session.id);
                            const recorder = new AuditRecorder(
                                {
                                    caller: {
                                        type: "subject",
                                        subject: principal.user.reference(
                                            Scope.universe.id,
                                            userId,
                                        ),
                                    },
                                    package: accountPackage,
                                    service: "account",
                                    scope: userId,
                                    sessionId,
                                },
                                this.journal,
                            );

                            // record the issued session's verified identity
                            await this.journal.database.transaction((transaction) =>
                                recorder.record(transaction, sessionCreated, {
                                    targets: {
                                        user: { type: "user", id: userId },
                                        session: { type: "session", id: sessionId },
                                    },
                                    details:
                                        context.path === undefined ? {} : { method: context.path },
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
        handler: (request: Request) => Promise<Response>,
    ): Promise<Response> {
        // match the request path against the audited routes
        const path = new URL(request.url).pathname.slice("/auth".length).split("/");
        const route = routes.find((route) => {
            const segments = route.split("/");

            return (
                segments.length === path.length &&
                segments.every(
                    (segment, index) => segment.startsWith(":") || segment === path[index],
                )
            );
        });
        if (!route) {
            return handler(request);
        }

        // propagate trace context without credentials
        return context.with(extractContext(request.headers), () =>
            telemetry.scope(accountPackage).tracer.startActiveSpan(
                `authentication ${route}`,
                {
                    kind: SpanKind.SERVER,
                    attributes: {
                        "http.route": `/auth${route}`,
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
        handler: (request: Request) => Promise<Response>,
        span: Span,
    ): Promise<Response> {
        // associate durable records with the verified caller and active trace
        const trace = span.spanContext();
        const recorder = new AuditRecorder(
            {
                caller,
                package: accountPackage,
                service: "account",
                scope: caller.type === "subject" ? caller.subject.id : Scope.universe.id,
                traceId: isSpanContextValid(trace) ? trace.traceId : undefined,
            },
            this.journal,
        );
        const attempt = READS.has(route)
            ? undefined
            : recorder.begin(authenticationRequest, {
                  targets: { endpoint: { type: "authentication-endpoint", id: route } },
                  details: { method: request.method },
              });

        // require durable acceptance before running a credential mutation
        if (attempt) {
            await recorder.append(attempt);
        }

        // preserve both failures if dispatch and its audit recording fail
        let response: Response;
        try {
            response = await handler(request);
        } catch (error) {
            span.setStatus({ code: SpanStatusCode.ERROR });
            if (attempt) {
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
                    );
                }
            }

            throw error;
        }

        // classify callback rejection without retaining redirect URLs or provider messages
        const location = response.headers.get("location");
        const rejected =
            response.status >= 400 ||
            (location !== null && new URL(location, request.url).searchParams.has("error"));
        span.setAttribute("http.response.status_code", response.status);
        if (rejected) {
            span.setStatus({ code: SpanStatusCode.ERROR });
        }

        // acknowledge the response only after its durable outcome
        if (attempt) {
            await recorder.append(
                recorder.finish(
                    attempt,
                    rejected
                        ? {
                              kind: "failure",
                              error: {
                                  code: "AUTHENTICATION_REJECTED",
                                  status: response.status,
                                  message: "authentication rejected",
                              },
                          }
                        : { kind: "success" },
                ),
            );
        }

        return response;
    }
}

import { AuditRecorder, defineAuditAction, type AuditActor } from "@destack/audit";
import { Scope } from "@destack/sync";
import { AuditOutbox } from "@destack/audit/outbox";
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
import type { DatabaseConnection } from "@destack/db";
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
    name: "Authentication.request",
    version: 1,
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
    name: "Session.create",
    version: 1,
    targets: schema.object({
        user: schema.object({ type: schema.literal("user"), id: identifier("user") }),
        session: schema.object({ type: schema.literal("session"), id: identifier("session") }),
    }),
    details: schema.object({
        /** The path of the endpoint issuing the session, absent for a server-only endpoint without one. */
        method: schema.string().optional(),
    }),
});

/** Record authentication HTTP outcomes through the persistent account outbox. */
export class AuthenticationAudit {
    /** The durable event writer, migrated by the host. */
    readonly outbox: AuditOutbox;

    /** Bind authentication records to the migrated account database. */
    constructor(database: DatabaseConnection) {
        this.outbox = new AuditOutbox(database);
    }

    /** Resolve the audit actor of a request. */
    static actor(current: { user: { id: string } } | null): AuditActor {
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
                                    actor: {
                                        type: "subject",
                                        subject: principal.user.reference(
                                            Scope.universe.id,
                                            userId,
                                        ),
                                    },
                                    delegation: [],
                                    package: accountPackage,
                                    service: "account",
                                    scope: userId,
                                    sessionId,
                                },
                                this.outbox,
                            );

                            // record the issued session's verified identity
                            await this.outbox.database.transaction((transaction) =>
                                recorder.record(transaction, sessionCreated, {
                                    targets: {
                                        user: { type: "user", id: userId },
                                        session: { type: "session", id: sessionId },
                                    },
                                    details:
                                        context.path === undefined ? {} : { method: context.path },
                                    outcome: "success",
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
        actor: AuditActor,
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
                        return await this.#record(request, route, actor, handler, span);
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
        actor: AuditActor,
        handler: (request: Request) => Promise<Response>,
        span: Span,
    ): Promise<Response> {
        // associate durable records with the verified actor and active trace
        const trace = span.spanContext();
        const recorder = new AuditRecorder(
            {
                actor,
                delegation: [],
                package: accountPackage,
                service: "account",
                scope: actor.type === "subject" ? actor.subject.id : Scope.universe.id,
                traceId: isSpanContextValid(trace) ? trace.traceId : undefined,
            },
            this.outbox,
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
                        recorder.complete(attempt, {
                            outcome: "failure",
                            errorCode: "INTERNAL_SERVER_ERROR",
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
                recorder.complete(
                    attempt,
                    rejected
                        ? { outcome: "failure", errorCode: "AUTHENTICATION_REJECTED" }
                        : { outcome: "success" },
                ),
            );
        }

        return response;
    }
}

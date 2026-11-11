import { CallErrorCode, Failure } from "@destack/error";
import { AuditCaller, defineAuditAction } from "@destack/object";
import type { Outcome } from "@destack/sync";
import { AuditRecorder, type Journal } from "@destack/object/server";
import { Scope } from "@destack/sync";
import { present, schema, zip } from "@destack/schema";
import {
    SpanKind,
    SpanStatusCode,
    SeverityNumber,
    TelemetryScope,
    extractContext,
    isSpanContextValid,
    type Span,
    trace,
} from "@destack/telemetry";
import type { BetterAuthPlugin } from "better-auth";
import { createAuthMiddleware } from "better-auth/api";
import { queueAfterTransactionHook } from "@better-auth/core/context";
import { principal, ANONYMOUS } from "@destack/access";
import type { Fetch } from "@destack/object";
import { sessionCreate } from "../object/session.ts";
import { PackageVersion } from "@destack/package";
import accountPackage from "../package.ts";
import { METADATA_PATHS } from "../authentication/path.ts";

/** The authentication reads, which leave credentials and challenges unchanged. */
const READS = new Set([
    "/get-session",
    "/list-accounts",
    "/passkey/list-user-passkeys",
    "/oauth2/userinfo",
    ...METADATA_PATHS,
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
            ? { subject: ANONYMOUS }
            : {
                  subject: principal.user.reference(
                      Scope.universe.id,
                      schema.identifier("user").parse(current.user.id),
                  ),
              };
    }

    /** Report protocol diagnostics with fixed messages and standard severity. */
    log(level: keyof typeof SEVERITIES): void {
        const { telemetry } = this.journal.environment;
        TelemetryScope.of(PackageVersion.of(accountPackage), telemetry).logger.emit({
            severityNumber: SEVERITIES[level],
            severityText: level.toUpperCase(),
            body: "authentication protocol diagnostic",
            context: telemetry.context.active(),
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
                                    package: PackageVersion.of(accountPackage),
                                    scope: userId,
                                    sessionId,
                                },
                                this.journal,
                            );

                            // record the issued session's verified identity once the request commits
                            await queueAfterTransactionHook(async () => {
                                await this.journal.database.transaction((transaction) =>
                                    recorder.record(transaction, sessionCreate, {
                                        target: { type: "session", id: sessionId },
                                        details:
                                            endpoint.path === undefined
                                                ? {}
                                                : { path: endpoint.path },
                                        outcome: { kind: "success" },
                                    }),
                                );
                            });
                        }),
                    },
                ],
            },
        };
    }

    /** Trace a request and record its outcome once it ends. */
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
                zip(segments, path).every(
                    ([segment, expected]) => segment.startsWith(":") || segment === expected,
                )
            );
        });
        if (route === undefined) {
            return handler(request);
        }

        // propagate trace context without credentials, the request's span active while it runs
        const { telemetry } = this.journal.environment;
        const parent = extractContext(request.headers, telemetry);
        const { tracer } = TelemetryScope.of(PackageVersion.of(accountPackage), telemetry);
        const span = tracer.startSpan(
            `authentication ${route}`,
            {
                kind: SpanKind.SERVER,
                attributes: {
                    "http.route": `${this.basePath}${route}`,
                    "http.request.method": request.method,
                },
            },
            parent,
        );

        return telemetry.context.with(trace.setSpan(parent, span), async () => {
            try {
                return await this.#record(request, route, caller, handler, span);
            } finally {
                span.end();
            }
        });
    }

    /** Record a request's outcome once it ends, a failure as an internal error. */
    async #record(
        request: Request,
        route: string,
        caller: AuditCaller,
        handler: Fetch,
        span: Span,
    ): Promise<Response> {
        // associate durable records with the verified caller and active trace
        const traced = span.spanContext();
        const recorder = new AuditRecorder(
            {
                caller,
                package: PackageVersion.of(accountPackage),
                scope: principal.user.is(caller.subject) ? caller.subject.id : Scope.universe.id,
                ...(isSpanContextValid(traced) ? { traceId: traced.traceId } : {}),
            },
            this.journal,
        );

        // answer reads without a record
        if (READS.has(route)) {
            return BetterAuthAudit.#traced(request, await handler(request), span);
        }

        // run the request, then record its outcome
        const attempt = recorder.begin(authenticationRequest, {
            target: { type: "authentication-endpoint", id: route },
            details: { method: request.method },
        });
        const response = await BetterAuthAudit.#dispatch(request, handler, recorder, attempt, span);
        await recorder.append(
            recorder.finish(attempt, BetterAuthAudit.#outcome(request, response)),
        );

        return BetterAuthAudit.#traced(request, response, span);
    }

    /** Run a request, and record its failure. */
    static async #dispatch(
        request: Request,
        handler: Fetch,
        recorder: AuditRecorder,
        attempt: ReturnType<AuditRecorder["begin"]>,
        span: Span,
    ): Promise<Response> {
        try {
            return await handler(request);
        } catch (error) {
            // record the failure
            span.setStatus({ code: SpanStatusCode.ERROR });
            try {
                await recorder.append(
                    recorder.finish(attempt, {
                        kind: "failure",
                        error: Failure.http("INTERNAL_SERVER_ERROR", "internal error"),
                    }),
                );
            } catch (auditError) {
                throw new AggregateError(
                    [error, auditError],
                    "authentication and audit recording failed",
                    { cause: auditError },
                );
            }
            throw error;
        }
    }

    /** Mark a response's status on its span, and a refusal as an error. */
    static #traced(request: Request, response: Response, span: Span): Response {
        span.setAttribute("http.response.status_code", response.status);
        if (BetterAuthAudit.#isRejected(request, response)) {
            span.setStatus({ code: SpanStatusCode.ERROR });
        }

        return response;
    }

    /** Read a response's outcome, a rejecting redirect as unauthorized. */
    static #outcome(request: Request, response: Response): Outcome {
        const code = response.status >= 400 ? CallErrorCode.of(response.status) : "UNAUTHORIZED";

        return BetterAuthAudit.#isRejected(request, response)
            ? { kind: "failure", error: Failure.http(code, "authentication rejected") }
            : { kind: "success" };
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

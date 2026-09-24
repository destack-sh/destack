import type { PackageId } from "@destack/package";
import type { AccessContext } from "@destack/access";
import type { ResourceContext } from "@destack/resource/context";
import { type Caller, CALLER_LIFETIME_MS } from "../authentication/index.ts";
import { ServiceError } from "../error/index.ts";

/** Verified identity and installation resources supplied to a user service invocation. */
export class ServiceContext {
    /** Incoming request and cancellation signal. */
    readonly request: Request;
    /** Receiving package identifier fixed by the hosting deployment. */
    readonly audience: PackageId;
    /** Authorization scope selected from verified credentials or host configuration. */
    readonly scope: string;
    /** Authenticated identity, or null for an anonymous request. */
    readonly caller: Caller | null;
    /** Resource clients bound by the host for this installation. */
    readonly resources: ResourceContext;
    /** Credential failure retained for procedure audit recording. */
    readonly authenticationError?: unknown;
    /** Server-generated correlation identity shared by request and domain audit events. */
    readonly requestId = crypto.randomUUID();
    /** Cancellation when the request closes or its verified identity expires. */
    readonly signal: AbortSignal;

    /** Retain host-selected scope independently of request input. */
    constructor(
        request: Request,
        audience: PackageId,
        scope: string,
        caller: Caller | null,
        resources: ResourceContext,
        authenticationError?: unknown,
    ) {
        // retain the request and its authenticated caller
        this.request = request;
        this.audience = audience;
        this.scope = scope;
        this.caller = caller;
        this.resources = resources;
        this.authenticationError = authenticationError;

        // preserve verified context methods when oRPC merges middleware context objects
        this.requireCaller = this.requireCaller.bind(this);
        this.access = this.access.bind(this);

        // retain cancellation as an own property across service middleware context copies
        const deadline =
            caller &&
            Math.min(
                caller.authentication.expiresAt,
                caller.authentication.verifiedAt + CALLER_LIFETIME_MS,
            );
        this.signal =
            deadline === null
                ? this.request.signal
                : AbortSignal.any([
                      this.request.signal,
                      AbortSignal.timeout(Math.max(0, Math.ceil(deadline - Date.now()))),
                  ]);
    }

    /** Require a current authenticated caller before performing identity-dependent work. */
    requireCaller(): Caller {
        // preserve invalid credentials and unavailable identity authorities
        if (this.authenticationError !== undefined) {
            throw this.authenticationError;
        }
        if (!this.caller) {
            throw new ServiceError("UNAUTHORIZED");
        }
        this.caller.context(this.audience, Date.now(), this.scope);

        return this.caller;
    }

    /** Read authorization inputs with a fresh time for each operation or stream event. */
    access(): AccessContext {
        if (this.authenticationError !== undefined) {
            throw this.authenticationError;
        }

        return this.caller
            ? this.caller.context(this.audience, Date.now(), this.scope)
            : { subjects: [], attributes: {}, now: Date.now() };
    }
}

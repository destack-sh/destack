import type { PackageId } from "@destack/package";
import type { AccessContext } from "@destack/access";
import type { ResourceContext } from "@destack/resource/context";
import type { Caller } from "../authentication/index.ts";
import { ServiceError } from "../error/index.ts";

/** Verified identity and installation resources supplied to a user service invocation. */
export class ServiceContext {
    /** Incoming request and cancellation signal. */
    readonly request: Request;
    /** Receiving package identifier fixed by the hosting deployment. */
    readonly audience: PackageId;
    /** Verified credential space, or the hosting space for an unscoped caller. */
    readonly spaceId: string;
    /** Authenticated identity, or null for an anonymous request. */
    readonly caller: Caller | null;
    /** Resource clients bound by the host for this installation. */
    readonly resources: ResourceContext;
    /** Credential failure retained for procedure audit recording. */
    readonly authenticationError?: unknown;
    /** Server-generated correlation identity shared by request and domain audit events. */
    readonly requestId = crypto.randomUUID();

    /** Retain host-selected scope independently of request input. */
    constructor(
        request: Request,
        audience: PackageId,
        spaceId: string,
        caller: Caller | null,
        resources: ResourceContext,
        authenticationError?: unknown,
    ) {
        this.request = request;
        this.audience = audience;
        this.spaceId = spaceId;
        this.caller = caller;
        this.resources = resources;
        this.authenticationError = authenticationError;
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
        this.caller.context(this.audience, Date.now(), this.spaceId);

        return this.caller;
    }

    /** Read authorization inputs with a fresh time for each operation or stream event. */
    get access(): AccessContext {
        if (this.authenticationError !== undefined) {
            throw this.authenticationError;
        }

        return this.caller
            ? this.caller.context(this.audience, Date.now(), this.spaceId)
            : { subjects: [], attributes: {}, now: Date.now() };
    }
}

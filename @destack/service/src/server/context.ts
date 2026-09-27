import type { PackageId } from "@destack/package";
import {
    Authorization,
    delegationChain,
    type AccessContext,
    type Authorizer,
    type ObjectReference,
} from "@destack/access";
import type { DatabaseConnection } from "@destack/db";
import type { ResourceContext } from "@destack/resource/context";
import { type Caller, CALLER_LIFETIME_MS } from "../authentication/index.ts";
import { ServiceError } from "../error/index.ts";
import type { ProcedureCall } from "./access.ts";
import { BOOKMARK_HEADER, Bookmark } from "../bookmark/index.ts";

/** The header carrying capabilities, such as link secrets, a request presents. */
export const CAPABILITY_HEADER = "destack-capability";

/** Verified identity and installation resources supplied to a user service invocation. */
export class ServiceContext {
    /** Incoming request and cancellation signal. */
    readonly request: Request;
    /** Receiving package identifier fixed by the hosting deployment. */
    readonly audience: PackageId;
    /** The call's scope, from host configuration or the verified caller; absent for anonymous calls to regional services. */
    readonly scope: string | undefined;
    /** Authenticated identity, or null for an anonymous request or a failed authentication. */
    readonly caller: Caller | null;
    /** Resource clients bound by the host for this installation. */
    readonly resources: ResourceContext;
    /** Credential failure retained for procedure audit recording. */
    readonly authenticationError?: unknown;
    /** Server-generated correlation identity shared by request and domain audit events. */
    readonly requestId = crypto.randomUUID();
    /** Cancellation when the request closes or its verified identity expires. */
    readonly signal: AbortSignal;
    /** Digests of the capabilities, such as link secrets, the request presented. */
    readonly capabilities: readonly string[];
    /** The watermarks the caller requires before this request reads. */
    readonly bookmark: Bookmark;
    /** The watermarks this request's writes reached, returned to the caller. */
    readonly observed = new Bookmark();
    /** The caller's authorization under the service's policies, for services that declare them. */
    readonly authorization: Authorization | undefined;

    /** Retain host-selected scope independently of request input. */
    constructor(request: Request, options: ServiceContextOptions) {
        // read the host-verified state
        const { audience, scope, caller, resources, access, authenticationError } = options;
        const capabilities = options.capabilities ?? [];

        // require a failed authentication to leave no caller
        if (caller !== null && authenticationError !== undefined) {
            throw new TypeError("a failed authentication has no caller");
        }

        // retain the request and its authenticated caller
        this.request = request;
        this.audience = audience;
        this.scope = scope;
        this.caller = caller;
        this.resources = resources;
        this.authenticationError = authenticationError;
        this.capabilities = capabilities;
        this.bookmark = Bookmark.parse(request.headers.get(BOOKMARK_HEADER));

        // authorize the caller under the service's policies, bound to each scope it acts in
        this.authorization =
            access &&
            new Authorization(access.authorizer, access.database, (scope) => {
                const context = this.access(scope);
                delegationChain(context);

                return context;
            });

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

        // require a caller whose authentication is still current
        if (!this.caller) {
            throw new ServiceError("UNAUTHORIZED");
        }
        this.caller.requireCurrent(this.audience, Date.now(), this.scope);

        return this.caller;
    }

    /** Read authorization inputs with a fresh time for each operation or stream event. */
    access(scope: string | undefined = this.scope): AccessContext {
        // preserve invalid credentials and unavailable identity authorities
        if (this.authenticationError !== undefined) {
            throw this.authenticationError;
        }

        // read the caller's current context, or an anonymous one, with the presented capabilities
        const context = this.caller
            ? this.caller.context(this.audience, Date.now(), scope)
            : { subjects: [], attributes: {}, now: Date.now() };

        return this.capabilities.length === 0
            ? context
            : { ...context, capabilities: this.capabilities };
    }
}

/** The host-verified state of one request. */
export interface ServiceContextOptions {
    /** The receiving package. */
    readonly audience: PackageId;
    /** The call's scope, from host configuration or the verified caller. */
    readonly scope?: string;
    /** The authenticated identity, or null for an anonymous request or a failed authentication. */
    readonly caller: Caller | null;
    /** The resource clients bound by the host. */
    readonly resources: ResourceContext;
    /** The service's policies, for services that declare them. */
    readonly access?: ServiceAccess;
    /** The credential failure, retained for procedure audit recording. */
    readonly authenticationError?: unknown;
    /** The digests of the capabilities the request presented. */
    readonly capabilities?: readonly string[];
}

/** How a service decides its protected procedures: its policies, the database holding their relationships, and each call's target. */
export interface ServiceAccess {
    /** The policies declaring every permission the service's procedures require. */
    readonly authorizer: Authorizer;
    /** The database whose relationships and roles decide each call. */
    readonly database: DatabaseConnection;
    /** Name the object a call acts on, or the scope it acts in, for procedures declaring a permission. */
    target?(call: ProcedureCall<ServiceContext>): Promise<ObjectReference>;
}

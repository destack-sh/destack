import { type ObjectReference } from "@destack/sync";
import type { PackageId } from "@destack/package";
import {
    Authorization,
    delegationChain,
    type AccessContext,
    type Authorizer,
} from "@destack/access";
import type { DatabaseConnection } from "@destack/db";
import type { ResourceContext } from "@destack/resource/context";
import type { Caller } from "../authentication/index.ts";
import { ServiceError } from "../error/index.ts";
import type { ProcedureCall } from "./access.ts";
import { BOOKMARK_HEADER, Bookmark } from "../bookmark/index.ts";

/** The header carrying a request's capabilities. */
export const CAPABILITY_HEADER = "destack-capability";

/** The verified context of a service call. */
export class ServiceContext {
    /** The request. */
    readonly request: Request;
    /** The receiving package. */
    readonly audience: PackageId;
    /** The call's scope. */
    readonly scope: string | undefined;
    /** The authenticated caller, or null. */
    readonly caller: Caller | null;
    /** The installation's resource clients. */
    readonly resources: ResourceContext;
    /** The credential failure. */
    readonly authenticationError?: unknown;
    /** The server-generated request identifier. */
    readonly requestId = crypto.randomUUID();
    /** Abort when the request closes or its caller lapses, ending handlers at their next consistent point. */
    readonly signal: AbortSignal;
    /** The digests of the presented capabilities. */
    readonly capabilities: readonly string[];
    /** The watermarks the caller requires. */
    readonly bookmark: Bookmark;
    /** The watermarks this request's writes reached. */
    readonly observed = new Bookmark();
    /** The caller's authorization under the service's policies. */
    readonly authorization: Authorization | undefined;
    /** The object the call's permission is decided on, set before the decision for audits of a denial. */
    target?: ObjectReference;

    /** Create the context of a request. */
    constructor(request: Request, options: ServiceContextOptions) {
        // read the host state
        const { audience, scope, caller, resources, access, authenticationError } = options;
        const capabilities = options.capabilities ?? [];

        // require no caller after a failed authentication
        if (caller !== null && authenticationError !== undefined) {
            throw new TypeError("a failed authentication has no caller");
        }

        // keep the request and caller
        this.request = request;
        this.audience = audience;
        this.scope = scope;
        this.caller = caller;
        this.resources = resources;
        this.authenticationError = authenticationError;
        this.capabilities = capabilities;
        this.bookmark = Bookmark.parse(request.headers.get(BOOKMARK_HEADER));

        // authorize the caller under the service's policies
        this.authorization =
            access &&
            new Authorization(access.authorizer, access.database, (scope) => {
                const context = this.access(scope);
                delegationChain(context);

                return context;
            });

        // bind methods for middleware context copies
        this.requireCaller = this.requireCaller.bind(this);
        this.access = this.access.bind(this);

        // keep the signal as an own property
        this.signal =
            caller === null
                ? this.request.signal
                : AbortSignal.any([
                      this.request.signal,
                      AbortSignal.timeout(Math.max(0, Math.ceil(caller.lapsesAt - Date.now()))),
                  ]);
    }

    /** Require an authenticated caller, current as of its lapse at latest. */
    requireCaller(): Caller {
        // report a credential failure
        if (this.authenticationError !== undefined) {
            throw this.authenticationError;
        }

        // require a current caller
        if (!this.caller) {
            throw new ServiceError("UNAUTHORIZED", { message: "missing caller credential" });
        }
        this.caller.requireCurrent(this.audience, this.caller.within(Date.now()), this.scope);

        return this.caller;
    }

    /** Read the access context at the current time, held at the caller's lapse once passed. */
    access(scope: string | undefined = this.scope): AccessContext {
        // report a credential failure
        if (this.authenticationError !== undefined) {
            throw this.authenticationError;
        }

        // read the caller's or an anonymous context with capabilities
        const context = this.caller
            ? this.caller.context(this.audience, this.caller.within(Date.now()), scope)
            : { subjects: [], attributes: {}, now: Date.now() };

        return this.capabilities.length === 0
            ? context
            : { ...context, capabilities: this.capabilities };
    }
}

/** The host-verified state of a request. */
export interface ServiceContextOptions {
    /** The receiving package. */
    readonly audience: PackageId;
    /** The call's scope. */
    readonly scope?: string;
    /** The authenticated caller, or null. */
    readonly caller: Caller | null;
    /** The resource clients bound by the host. */
    readonly resources: ResourceContext;
    /** The service's policies. */
    readonly access?: ServiceAccess;
    /** The credential failure. */
    readonly authenticationError?: unknown;
    /** The digests of the presented capabilities. */
    readonly capabilities?: readonly string[];
}

/** The policies and database that decide a service's calls. */
export interface ServiceAccess {
    /** The policies. */
    readonly authorizer: Authorizer;
    /** The database holding relationships and roles. */
    readonly database: DatabaseConnection;
    /** Name the object or scope a call acts on. */
    target?(call: ProcedureCall<ServiceContext>): Promise<ObjectReference>;
}

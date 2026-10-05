import { principal } from "@destack/access";
import { type ObjectReference, type Subject } from "@destack/sync";
import type { PackageId } from "@destack/package";
import {
    Authorization,
    AccessContext,
    type Authorizer,
    Caller,
    type Standing,
} from "@destack/access";
import type { DatabaseConnection } from "@destack/db";
import type { ResourceContext } from "@destack/resource/context";
import type { Authentication } from "../authentication/index.ts";
import { ServiceError } from "../error/index.ts";
import type { ProcedureCall } from "./access.ts";
import { BOOKMARK_HEADER, Bookmark } from "../bookmark/index.ts";

/** The header with the link secrets a request presents. */
export const LINK_SECRET_HEADER = "destack-link-secret";

/** The verified context of a service call. */
export class ServiceContext {
    /** The request. */
    readonly request: Request;
    /** The receiving package. */
    readonly audience: PackageId;
    /** The call's scope. */
    readonly scope: string | undefined;
    /** The caller's authentication, or null for an anonymous request. */
    readonly authentication: Authentication | null;
    /** The installation's resource clients. */
    readonly resources: ResourceContext;
    /** The credential failure, absent after a successful or anonymous authentication. */
    readonly authenticationError: Error | undefined;
    /** The server-generated request identifier. */
    readonly requestId = crypto.randomUUID();
    /** Abort when the request closes or its caller lapses, ending handlers at their next consistent point. */
    readonly signal: AbortSignal;
    /** The digests of the presented link secrets. */
    readonly linkSecrets: readonly string[];
    /** The watermarks the caller requires. */
    readonly bookmark: Bookmark;
    /** Read the current time calls run at. */
    readonly clock: () => number;
    /** The watermarks this request's writes reached. */
    readonly observed = new Bookmark();
    /** The caller's authorization under the service's policies. */
    readonly authorization: Authorization | undefined;
    /** The object the call's permission is decided on, set before the decision for audits of a denial. */
    target?: ObjectReference;

    /** Create the context of a request. */
    constructor(request: Request, options: ServiceContextOptions) {
        // read the host state
        const { audience, scope, authentication, resources, access, authenticationError } = options;
        const linkSecrets = options.linkSecrets ?? [];

        // require no caller after a failed authentication
        if (authentication !== null && authenticationError !== undefined) {
            throw new TypeError("a failed authentication leaves no authentication");
        }

        // keep the request and caller
        this.request = request;
        this.audience = audience;
        this.scope = scope;
        this.authentication = authentication;
        this.resources = resources;
        this.authenticationError = authenticationError;
        this.linkSecrets = linkSecrets;
        this.clock = options.clock ?? Date.now;
        this.bookmark = Bookmark.parse(request.headers.get(BOOKMARK_HEADER));

        // authorize the caller under the service's policies
        this.authorization =
            access &&
            new Authorization(access.authorizer, access.database, (authorizedScope) => {
                const context = this.access(authorizedScope);
                Caller.delegation(context);

                return context;
            });

        // bind methods for middleware context copies
        this.requireAuthentication = this.requireAuthentication.bind(this);
        this.requireAuthorization = this.requireAuthorization.bind(this);
        this.access = this.access.bind(this);

        // keep the signal as an own property
        this.signal =
            authentication === null
                ? this.request.signal
                : AbortSignal.any([
                      this.request.signal,
                      AbortSignal.timeout(
                          Math.max(0, Math.ceil(authentication.lapsesAt - this.clock())),
                      ),
                  ]);
    }

    /** Require an installation caller to hold a grant of a procedure from its space, concealing the procedure from one without it. */
    requireCall(path: string): Authentication {
        // admit callers other than installations by their own authority
        const authentication = this.requireAuthentication();
        const { subject, calls } = authentication.claims;
        if (!principal.installation.is(subject) || calls?.includes(path) === true) {
            return authentication;
        }

        // refuse an installation its space granted no such call
        throw new ServiceError("NOT_FOUND", { message: `no procedure ${path}` });
    }

    /** Require the caller's authentication, current as of its lapse at latest. */
    requireAuthentication(): Authentication {
        // report a credential failure
        if (this.authenticationError !== undefined) {
            throw this.authenticationError;
        }

        // require a current caller
        if (!this.authentication) {
            throw new ServiceError("UNAUTHORIZED", { message: "missing caller credential" });
        }
        this.authentication.requireCurrent(
            this.audience,
            this.authentication.within(this.clock()),
            this.scope,
        );

        return this.authentication;
    }

    /** Read the caller's authorization, which a service declared with access always has. */
    requireAuthorization(): Authorization {
        if (this.authorization === undefined) {
            throw new TypeError(`service ${this.audience} runs without authorization`);
        }

        return this.authorization;
    }

    /** Read the access context at the current time, fixed at the caller's lapse once passed. */
    access(scope: string | undefined = this.scope): AccessContext {
        // report a credential failure
        if (this.authenticationError !== undefined) {
            throw this.authenticationError;
        }

        // read the caller's or an anonymous context with link secrets
        const context = this.authentication
            ? this.authentication.context(
                  this.audience,
                  this.authentication.within(this.clock()),
                  scope,
              )
            : { subjects: [], attributes: {}, now: this.clock() };

        return this.linkSecrets.length === 0
            ? context
            : { ...context, linkSecrets: this.linkSecrets };
    }
}

/** The host-verified state of a request. */
export interface ServiceContextOptions {
    /** The receiving package. */
    readonly audience: PackageId;
    /** The call's scope. */
    readonly scope?: string;
    /** The caller's authentication, or null for an anonymous request. */
    readonly authentication: Authentication | null;
    /** The resource clients bound by the host. */
    readonly resources: ResourceContext;
    /** The service's policies. */
    readonly access?: ServiceAccess;
    /** The credential failure. */
    readonly authenticationError?: Error;
    /** The digests of the presented link secrets. */
    readonly linkSecrets?: readonly string[];
    /** Read the current time calls run at, the system clock by default. */
    readonly clock?: () => number;
}

/** The policies and database that decide a service's calls. */
export interface ServiceAccess {
    /** The policies. */
    readonly authorizer: Authorizer;
    /** The database with relationships and roles. */
    readonly database: DatabaseConnection;
    /** Name the object or scope a call acts on. */
    target?(call: ProcedureCall<ServiceContext>): Promise<ObjectReference>;
    /** Find the object standing for a principal callers act as, absent when none stands for it here. */
    standing?(subject: Subject): Promise<Standing | undefined>;
}

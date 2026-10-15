import type { ObjectReference } from "@destack/sync";
import type { PackageId } from "@destack/package";
import {
    AccessContext,
    type Authentication,
    Authorization,
    type Authorizer,
    Caller,
    Credential,
} from "@destack/access";
import { type DatabaseConnection, Transaction } from "@destack/db";
import type { ResourceContext } from "@destack/resource/context";
import type { ProcedureCall } from "./procedure-call.ts";
import { BOOKMARK_HEADER, Bookmark } from "../bookmark/index.ts";
import { Clock } from "@destack/schema";
import { CallError } from "../error/index.ts";

/** The header with the link secrets a request presents. */
export const LINK_SECRET_HEADER = "Destack-Link-Secret";

/** The verified context of a call. */
export class RequestContext {
    /** The request. */
    readonly request: Request;
    /** The receiving package. */
    readonly audience: PackageId;
    /** The call's scope. */
    readonly scope: string | undefined;
    /** The installation's resource clients. */
    readonly resources: ResourceContext;
    /** The credential failure, absent after a successful or anonymous authentication. */
    readonly authenticationError: Error | undefined;
    /** The server-generated request identifier. */
    readonly requestId = crypto.randomUUID();
    /** Abort when the request closes, its caller lapses or the server drains. */
    readonly signal: AbortSignal;
    /** The digests of the presented link secrets. */
    readonly linkSecrets: readonly string[];
    /** The watermarks the caller requires. */
    readonly bookmark: Bookmark;
    /** The clock calls run at. */
    readonly clock: Clock;
    /** The watermarks this request's writes reached. */
    readonly observed = new Bookmark();
    /** The caller's authorization under the package's policies. */
    readonly authorization: Authorization | undefined;
    /** The object the call's permission is decided on. */
    target?: ObjectReference;
    /** The verified caller with its credential, absent for an anonymous request. */
    readonly authentication: Authentication | undefined;

    /** Create the context of a request. */
    constructor(request: Request, options: RequestContextOptions) {
        // read the host state
        const { audience, scope, authentication, resources, access, authenticationError } = options;
        const linkSecrets = options.linkSecrets ?? [];

        // require no caller after a failed authentication
        if (authentication !== undefined && authenticationError !== undefined) {
            throw new TypeError("a failed authentication has no caller");
        }

        // keep the request and caller
        this.request = request;
        this.audience = audience;
        this.scope = scope;
        this.authentication = authentication;
        this.resources = resources;
        this.authenticationError = authenticationError;
        this.linkSecrets = linkSecrets;
        this.clock = options.clock;
        this.bookmark = Bookmark.parse(request.headers.get(BOOKMARK_HEADER));

        // authorize the current caller in the request's scope under the package's policies
        this.authorization =
            access &&
            new Authorization(access.authorizer, Transaction.reading(access.database), () => {
                const authorized = this.access();
                Caller.requireChain(authorized);

                return authorized;
            });

        // bind methods for cloned middleware contexts
        this.requireAuthorization = this.requireAuthorization.bind(this);
        this.caller = this.caller.bind(this);
        this.credential = this.credential.bind(this);
        this.access = this.access.bind(this);

        // end the signal as the request closes, the credential lapses or the server drains
        const lapse = authentication?.credential.expiresAt;
        const lapsed = new AbortController();
        if (lapse !== undefined) {
            this.clock.sleep(lapse - this.clock.now(), this.request.signal).then(
                () => lapsed.abort(new DOMException("the credential lapsed", "TimeoutError")),
                () => {},
            );
        }
        this.signal = AbortSignal.any([
            this.request.signal,
            lapsed.signal,
            ...(options.draining === undefined ? [] : [options.draining]),
        ]);
    }

    /** Read the caller's authorization, present for every package declared with access. */
    requireAuthorization(): Authorization {
        if (this.authorization === undefined) {
            throw new TypeError(`package ${this.audience} has no authorization`);
        }

        return this.authorization;
    }

    /** Read the request's caller, anonymous without one, and throw a credential failure. */
    caller(): Caller {
        // report a credential failure
        if (this.authenticationError !== undefined) {
            throw this.authenticationError;
        }

        return this.authentication?.caller ?? Caller.anonymous();
    }

    /** Read the credential the request presented, and throw for an anonymous or failed one. */
    credential(): Credential {
        // report a credential failure, and refuse a request without a credential
        const credential = this.authentication?.credential;
        if (this.authenticationError !== undefined) {
            throw this.authenticationError;
        } else if (credential === undefined) {
            throw new CallError("UNAUTHORIZED", { message: "missing caller credential" });
        }

        return credential;
    }

    /** Read the request's access context in a scope, its credential current there. */
    access(scope: string | undefined = this.scope): AccessContext {
        // decide an anonymous caller now
        const caller = this.caller();
        const credential = this.authentication?.credential;
        const { linkSecrets } = this;
        if (credential === undefined) {
            return AccessContext.of(caller, this.clock.now(), { linkSecrets });
        }

        // decide a credential's caller before it lapses, the credential current in the scope
        const now = Credential.clamp(credential, this.clock.now());
        Credential.requireCurrent(credential, this.audience, now, scope);

        return AccessContext.of(caller, now, { credential, linkSecrets });
    }
}

/** The host-verified state of a request. */
export interface RequestContextOptions {
    /** The receiving package. */
    readonly audience: PackageId;
    /** The call's scope. */
    readonly scope?: string;
    /** The verified caller with its credential, absent for an anonymous request. */
    readonly authentication?: Authentication;
    /** The resource clients bound by the host. */
    readonly resources: ResourceContext;
    /** The package's policies. */
    readonly access?: ServerAccess;
    /** The credential failure. */
    readonly authenticationError?: Error;
    /** The digests of the presented link secrets. */
    readonly linkSecrets?: readonly string[];
    /** The clock calls run at. */
    readonly clock: Clock;
    /** Abort once the server drains, ending streams at their next consistent point. */
    readonly draining?: AbortSignal;
}

/** The policies and database that decide a package's calls. */
export interface ServerAccess {
    /** The policies. */
    readonly authorizer: Authorizer;
    /** The database with relationships and roles. */
    readonly database: DatabaseConnection;
    /** Resolve the object or scope a call acts on. */
    readonly target?: (call: ProcedureCall<RequestContext>) => Promise<ObjectReference>;
}

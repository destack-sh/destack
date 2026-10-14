import { principal } from "@destack/access";
import type { ObjectReference, Subject } from "@destack/sync";
import type { PackageId } from "@destack/package";
import { AccessContext, Authorization, type Authorizer, Caller } from "@destack/access";
import { type DatabaseConnection, Transaction } from "@destack/db";
import type { ResourceContext } from "@destack/resource/context";
import { CallError } from "../error/index.ts";
import type { ProcedureCall } from "./procedure-call.ts";
import { BOOKMARK_HEADER, Bookmark } from "../bookmark/index.ts";
import { Clock } from "@destack/schema";

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
    /** The verified caller the request or its host presented, for any package, absent for none. */
    readonly presented: Caller | undefined;

    /** Create the context of a request. */
    constructor(request: Request, options: RequestContextOptions) {
        // read the host state
        const { audience, scope, caller, resources, access, authenticationError } = options;
        const linkSecrets = options.linkSecrets ?? [];

        // require no caller after a failed authentication
        if (caller !== undefined && authenticationError !== undefined) {
            throw new TypeError("a failed authentication has no caller");
        }

        // keep the request and caller
        this.request = request;
        this.audience = audience;
        this.scope = scope;
        this.presented = options.presented ?? caller;
        this.resources = resources;
        this.authenticationError = authenticationError;
        this.linkSecrets = linkSecrets;
        this.clock = options.clock;
        this.bookmark = Bookmark.parse(request.headers.get(BOOKMARK_HEADER));

        // authorize the current caller in the request's scope under the package's policies
        this.authorization =
            access &&
            new Authorization(access.authorizer, Transaction.reading(access.database), () => {
                const authorized = this.caller();
                Caller.requireChain(authorized);

                return AccessContext.of(authorized, this.clock.now());
            });

        // bind methods for cloned middleware contexts
        this.requireCaller = this.requireCaller.bind(this);
        this.requireAuthorization = this.requireAuthorization.bind(this);
        this.caller = this.caller.bind(this);

        // end the signal when the request closes, the credential lapses or the server drains
        const lapse = caller?.credential?.expiresAt;
        this.signal = AbortSignal.any([
            this.request.signal,
            ...(lapse === undefined
                ? []
                : [AbortSignal.timeout(Math.max(0, Math.ceil(lapse - this.clock.now())))]),
            ...(options.draining === undefined ? [] : [options.draining]),
        ]);
    }

    /** Require an installation caller to have a grant of a procedure from its space. */
    requireCall(path: string): Caller {
        // admit callers other than installations by their own authority
        const caller = this.requireCaller();
        const { subject, calls } = caller;
        if (!principal.installation.is(subject) || calls?.includes(path) === true) {
            return caller;
        }

        // refuse an installation its space granted no such call
        throw new CallError("NOT_FOUND", { message: `no procedure ${path}` });
    }

    /** Require a verified principal calling, current as of its credential's lapse at latest. */
    requireCaller(): Caller & { readonly subject: Subject } {
        // report a credential failure
        if (this.authenticationError !== undefined) {
            throw this.authenticationError;
        }

        // require a current principal
        const presented = this.presented;
        const subject = presented?.subject;
        if (presented === undefined || subject === undefined) {
            throw new CallError("UNAUTHORIZED", { message: "missing caller credential" });
        }
        const caller = { ...presented, subject };
        Caller.requireCurrent(
            caller,
            this.audience,
            Caller.clamp(caller, this.clock.now()),
            this.scope,
        );

        return caller;
    }

    /** Read the caller's authorization, present for every package declared with access. */
    requireAuthorization(): Authorization {
        if (this.authorization === undefined) {
            throw new TypeError(`package ${this.audience} has no authorization`);
        }

        return this.authorization;
    }

    /** Read the request's verified caller in a scope, with the link secrets it presented. */
    caller(scope: string | undefined = this.scope): Caller {
        // report a credential failure
        if (this.authenticationError !== undefined) {
            throw this.authenticationError;
        }

        // read the caller current in the scope with its link secrets
        const caller = this.presented ?? { subjects: [] };
        Caller.requireCurrent(caller, this.audience, Caller.clamp(caller, this.clock.now()), scope);

        return this.linkSecrets.length === 0
            ? caller
            : { ...caller, linkSecrets: this.linkSecrets };
    }
}

/** The host-verified state of a request. */
export interface RequestContextOptions {
    /** The receiving package. */
    readonly audience: PackageId;
    /** The call's scope. */
    readonly scope?: string;
    /** The verified caller, absent for an anonymous request. */
    readonly caller?: Caller;
    /** The verified caller as presented, which a mount forwarding below the package hands on. */
    readonly presented?: Caller;
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
    target?(call: ProcedureCall<RequestContext>): Promise<ObjectReference>;
}

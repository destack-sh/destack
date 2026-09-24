import type { PackageId } from "@destack/package";
import { type AccessContext, type Subject, type Delegation, sameSubject } from "@destack/access";
import { ServiceError } from "../error/index.ts";
import { identifier, schema } from "@destack/schema";

/** Maximum age of externally verified identity and membership records. */
export const CALLER_LIFETIME_MS = 60000;
/** Maximum difference between the authority's clock and the receiving service's clock. */
export const CALLER_CLOCK_TOLERANCE_MS = 5000;

/** Verified caller state established by the receiving host's credential verifier. */
export class Caller<Credential = unknown> {
    /** Host-verified authentication result; never accepted directly from request input. */
    readonly authentication: CallerAuthentication<Credential>;

    /** Retain a verified credential, its audience and its current subject relationships. */
    constructor(authentication: CallerAuthentication<Credential>) {
        this.authentication = structuredClone(authentication);
    }

    /** Read the credential reference used for authoritative local rechecks. */
    get credential(): Credential {
        return this.authentication.credential;
    }

    /** Qualify retry identity by both the acting and represented subjects. */
    get id(): string {
        const { actor, subject } = this.authentication;
        const acting = actor ?? subject;

        return JSON.stringify([
            subject.authority,
            subject.kind,
            subject.id,
            acting.authority,
            acting.kind,
            acting.id,
        ]);
    }

    /** Build a current access context after enforcing audience and authentication freshness. */
    context(audience: PackageId, now = Date.now(), scope?: string): AccessContext {
        // reject a different audience or scope and stale authentication
        const authentication = this.authentication;
        if (
            authentication.audience !== audience ||
            (authentication.scope !== undefined && authentication.scope !== scope) ||
            !Number.isFinite(authentication.verifiedAt) ||
            !Number.isFinite(authentication.expiresAt) ||
            authentication.verifiedAt > now + CALLER_CLOCK_TOLERANCE_MS ||
            now >= authentication.expiresAt ||
            now >= authentication.verifiedAt + CALLER_LIFETIME_MS
        ) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "caller authentication is expired or has a different audience or scope",
            });
        }

        // require the represented subject among the verified identities used by the evaluator
        if (
            !authentication.subjects.some((subject) => sameSubject(subject, authentication.subject))
        ) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "caller subject is missing from verified identities",
            });
        }

        return {
            subject: authentication.subject,
            actor: authentication.actor,
            subjects: authentication.subjects,
            delegations: authentication.delegations,
            attributes: authentication.attributes ?? {},
            permissions: authentication.permissions,
            now,
        };
    }
}

/** Credential verification performed locally or by an authenticated authoritative service. */
export interface CallerAuthentication<Credential = unknown> {
    /** Exact authority scope when the credential is scoped. */
    readonly scope?: string;
    /** Scope-qualified credential restrictions; an empty list grants no permissions. */
    readonly permissions?: AccessContext["permissions"];
    /** Credential reference for subsequent authoritative checks, excluding raw secrets. */
    readonly credential: Credential;
    /** Receiving package's immutable identifier. */
    readonly audience: PackageId;
    /** Verification time in Unix milliseconds, established by the credential authority. */
    readonly verifiedAt: number;
    /** Exclusive expiry in Unix milliseconds, bounded by the credential and assertion. */
    readonly expiresAt: number;
    /** Represented user or software identity. */
    readonly subject: Subject;
    /** Acting software when using delegated authority. */
    readonly actor?: Subject;
    /** Verified subject and group identities. */
    readonly subjects: readonly Subject[];
    /** Current delegation restrictions from the authenticating authority. */
    readonly delegations?: readonly Delegation[];
    /** Verified account memberships used by regional role mappings. */
    readonly memberships?: readonly CallerMembership[];
    /** Verified deployments for represented and acting workload identities. */
    readonly deployments?: readonly CallerDeployment[];
    /** Trusted attributes used by declared access policies. */
    readonly attributes?: AccessContext["attributes"];
}

/** An account membership verified by its global authority. */
export interface CallerMembership {
    /** User receiving the membership. */
    readonly subject: Subject;
    /** Administering account. */
    readonly accountId: schema.Infer<ReturnType<typeof identifier<"account">>>;
    /** Current account membership identifier. */
    readonly id: schema.Infer<ReturnType<typeof identifier<"account-membership">>>;
}

/** A workload identity authenticated within one deployment. */
export interface CallerDeployment {
    /** Represented or acting workload identity. */
    readonly subject: Subject;
    /** Exact authenticated deployment. */
    readonly id: schema.Infer<ReturnType<typeof identifier<"deployment">>>;
}

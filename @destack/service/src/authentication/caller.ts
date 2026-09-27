import type { PackageId } from "@destack/package";
import {
    type AccessContext,
    type Delegate,
    Subject,
    isPrincipal,
    principal,
    sameSubject,
    subjectKey,
    type AuthenticationAssurance,
    type VerifiedIdentifier,
} from "@destack/access";
import { identifier, schema } from "@destack/schema";
import { ServiceError } from "../error/index.ts";
import type { TokenIssuerAuthority } from "./token.ts";

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

    /** Qualify retry identity by both the represented subject and the principal sending the request. */
    get id(): string {
        const { delegates, subject } = this.authentication;
        const sending = delegates?.at(-1)?.subject ?? subject;

        return JSON.stringify([subjectKey(subject), subjectKey(sending)]);
    }

    /** Require authentication for the audience and scope that is still fresh at a time. */
    requireCurrent(audience: PackageId, now: number, scope?: string): void {
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
    }

    /** Build a current access context after enforcing audience and authentication freshness. */
    context(audience: PackageId, now = Date.now(), scope?: string): AccessContext {
        this.requireCurrent(audience, now, scope);
        const authentication = this.authentication;

        return {
            subject: authentication.subject,
            subjects: authentication.subjects,
            ...(authentication.assurance === undefined
                ? {}
                : { assurance: authentication.assurance }),
            ...(authentication.identifiers === undefined
                ? {}
                : { identifiers: authentication.identifiers }),
            ...(isCredentialReference(authentication.credential)
                ? { session: `${authentication.credential.kind}:${authentication.credential.id}` }
                : {}),
            delegates: authentication.delegates,
            attributes: authentication.attributes ?? {},
            permissions: authentication.permissions,
            now,
        };
    }

    /** Require identity claims within an issuer's authority, with bound deployments and a valid delegation chain. */
    requireAuthority(authority: TokenIssuerAuthority): void {
        // collect every asserted identity
        const authentication = this.authentication;
        const subjects = [authentication.subject, ...authentication.subjects];

        // allow a space signing key to assert only installations of its own space
        if (
            authority.kind === "space" &&
            (authentication.scope !== authority.spaceId ||
                authentication.identifiers?.length ||
                authentication.delegates?.length ||
                subjects.some(
                    (subject) =>
                        !principal.installation.is(subject) || subject.scope !== authority.spaceId,
                ))
        ) {
            throw new ServiceError("UNAUTHORIZED", { message: "token exceeds issuer authority" });
        }

        // bind the installations of the token's space, deployed packages, to exactly one authenticated deployment
        const workloads = [
            ...subjects,
            ...(authentication.delegates ?? []).map((delegate) => delegate.subject),
        ].filter(
            (subject) =>
                principal.installation.is(subject) && subject.scope === authentication.scope,
        );
        const deployments = authentication.deployments ?? [];
        for (const subject of workloads) {
            const matching = deployments.filter((deployment) =>
                sameSubject(deployment.subject, subject),
            ).length;
            if (matching !== 1) {
                throw new ServiceError("UNAUTHORIZED", {
                    message: "invalid workload token identity",
                });
            }
        }

        // reject deployments of no asserted workload
        if (
            deployments.some(
                (deployment) =>
                    !workloads.some((subject) => sameSubject(subject, deployment.subject)),
            )
        ) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "unexpected workload token identity",
            });
        }

        // require an acyclic chain of single principals, where only a user impersonating the subject acts with its full authority
        const delegates = authentication.delegates ?? [];
        const chain = [authentication.subject, ...delegates.map((delegate) => delegate.subject)];
        if (
            delegates.some(
                (delegate, position) =>
                    !isPrincipal(delegate.subject) ||
                    (delegate.authority === "full" &&
                        (position !== 0 || !principal.user.is(delegate.subject))),
            ) ||
            chain.some((entry, position) =>
                chain.slice(position + 1).some((other) => sameSubject(entry, other)),
            )
        ) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid token delegation" });
        }
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
    /** How strongly and how recently the represented subject authenticated. */
    readonly assurance?: AuthenticationAssurance;
    /** Identifiers, such as email addresses, the represented subject proved control of. */
    readonly identifiers?: readonly VerifiedIdentifier[];
    /** The verified principals and the subject sets, such as account members, the caller belongs to. */
    readonly subjects: readonly Subject[];
    /** The principals acting in order, each for the one before and the first for the subject; the last sends the request. */
    readonly delegates?: readonly Delegate[];
    /** Verified deployments for represented and acting workload identities. */
    readonly deployments?: readonly CallerDeployment[];
    /** Trusted attributes used by declared access policies. */
    readonly attributes?: AccessContext["attributes"];
}

/** A workload identity authenticated within one deployment. */
export const CallerDeployment = schema.object({
    /** Represented or acting workload identity. */
    subject: Subject,
    /** Exact authenticated deployment. */
    id: identifier("deployment"),
});
/** A workload identity authenticated within one deployment. */
export type CallerDeployment = schema.Infer<typeof CallerDeployment>;

/** Whether a credential names its kind and identifier, identifying the session it authenticates. */
function isCredentialReference(credential: unknown): credential is { kind: string; id: string } {
    return (
        typeof credential === "object" &&
        credential !== null &&
        typeof (credential as { kind?: unknown }).kind === "string" &&
        typeof (credential as { id?: unknown }).id === "string"
    );
}

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

/** The maximum age of verified identity and membership records. */
export const CALLER_LIFETIME_MILLISECONDS = 60000;
/** The maximum clock difference between the authority and the receiving service. */
export const CALLER_CLOCK_TOLERANCE_MILLISECONDS = 5000;

/** A verified caller. */
export class Caller<Credential = unknown> {
    /** The host-verified authentication. */
    readonly authentication: CallerAuthentication<Credential>;

    /** Create the caller from its verified authentication. */
    constructor(authentication: CallerAuthentication<Credential>) {
        this.authentication = structuredClone(authentication);
    }

    /** The credential reference. */
    get credential(): Credential {
        return this.authentication.credential;
    }

    /** The retry identity of the subject and the sending principal. */
    get id(): string {
        const { delegates, subject } = this.authentication;
        const sending = delegates?.at(-1)?.subject ?? subject;

        return JSON.stringify([subjectKey(subject), subjectKey(sending)]);
    }

    /** Require current authentication for the audience and scope. */
    requireCurrent(audience: PackageId, now: number, scope?: string): void {
        // reject another audience or scope and stale authentication
        const authentication = this.authentication;
        if (
            authentication.audience !== audience ||
            (authentication.scope !== undefined && authentication.scope !== scope) ||
            !Number.isFinite(authentication.verifiedAt) ||
            !Number.isFinite(authentication.expiresAt) ||
            authentication.verifiedAt > now + CALLER_CLOCK_TOLERANCE_MILLISECONDS ||
            now >= authentication.expiresAt ||
            now >= authentication.verifiedAt + CALLER_LIFETIME_MILLISECONDS
        ) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "caller authentication is expired or has a different audience or scope",
            });
        }

        // require the subject among the verified identities
        if (
            !authentication.subjects.some((subject) => sameSubject(subject, authentication.subject))
        ) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "caller subject is missing from verified identities",
            });
        }
    }

    /** Build an access context for the audience after checking freshness. */
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

    /** Require identity claims within an issuer's authority. */
    requireAuthority(authority: TokenIssuerAuthority): void {
        // collect every asserted identity
        const authentication = this.authentication;
        const subjects = [authentication.subject, ...authentication.subjects];

        // allow a space key to assert only installations of its space
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

        // bind each installation of the token's space to one deployment
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

        // require an acyclic chain of single principals
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

/** A verified credential. */
export interface CallerAuthentication<Credential = unknown> {
    /** The authority scope of a scoped credential. */
    readonly scope?: string;
    /** The credential's permission restrictions. */
    readonly permissions?: AccessContext["permissions"];
    /** The credential reference. */
    readonly credential: Credential;
    /** The receiving package. */
    readonly audience: PackageId;
    /** The verification time, in Unix milliseconds. */
    readonly verifiedAt: number;
    /** The exclusive expiry, in Unix milliseconds. */
    readonly expiresAt: number;
    /** The represented user or software identity. */
    readonly subject: Subject;
    /** How strongly and how recently the subject authenticated. */
    readonly assurance?: AuthenticationAssurance;
    /** The identifiers the subject proved control of. */
    readonly identifiers?: readonly VerifiedIdentifier[];
    /** The verified principals and the subject sets the caller belongs to. */
    readonly subjects: readonly Subject[];
    /** The acting principals in order, the last sending the request. */
    readonly delegates?: readonly Delegate[];
    /** The verified deployments of workload identities. */
    readonly deployments?: readonly CallerDeployment[];
    /** The trusted attributes access policies read. */
    readonly attributes?: AccessContext["attributes"];
}

/** A workload identity authenticated within one deployment. */
export const CallerDeployment = schema.object({
    /** The workload identity. */
    subject: Subject,
    /** The deployment. */
    id: identifier("deployment"),
});
/** A workload identity authenticated within one deployment. */
export type CallerDeployment = schema.Infer<typeof CallerDeployment>;

/** Whether a credential has a kind and an identifier. */
function isCredentialReference(credential: unknown): credential is { kind: string; id: string } {
    return (
        typeof credential === "object" &&
        credential !== null &&
        typeof (credential as { kind?: unknown }).kind === "string" &&
        typeof (credential as { id?: unknown }).id === "string"
    );
}

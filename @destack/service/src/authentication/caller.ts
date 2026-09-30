import { PackageId } from "@destack/package";
import {
    type AccessContext,
    Attribute,
    AuthenticationAssurance,
    Delegate,
    isPrincipal,
    PermissionReference,
    principal,
    sameSubject,
    Subject,
    subjectKey,
    VerifiedIdentifier,
} from "@destack/access";
import { identifier, schema } from "@destack/schema";
import { ServiceError } from "../error/index.ts";
import type { TokenIssuerAuthority } from "./token.ts";

/** The maximum age of verified identity and membership records. */
export const CALLER_LIFETIME_MILLISECONDS = 60000;
/** The maximum clock difference between the authority and the receiving service. */
export const CALLER_CLOCK_TOLERANCE_MILLISECONDS = 5000;

/** A workload identity authenticated within one deployment. */
export const CallerDeployment = schema.object({
    /** The workload identity. */
    subject: Subject,
    /** The deployment. */
    id: identifier("deployment"),
});
/** A workload identity authenticated within one deployment. */
export type CallerDeployment = schema.Infer<typeof CallerDeployment>;

/** A permission a credential or delegation step keeps. */
const KeptPermission = PermissionReference.extend({
    /** The authority scope. */
    scope: schema.string().min(1),
    /** The object restriction. */
    objectId: schema.string().optional(),
});

/** A verified caller in transit between hosts, runners and services. */
export const CallerAuthentication = schema.object({
    /** The authority scope of a scoped credential. */
    scope: schema.string().min(1).optional(),
    /** The credential's permission restrictions. */
    permissions: schema.array(KeptPermission).optional(),
    /** The credential reference. */
    credential: schema.object({
        /** The credential kind. */
        kind: schema.string().min(1),
        /** The credential identifier. */
        id: schema.string().min(1),
    }),
    /** The receiving package. */
    audience: PackageId,
    /** The verification time, in Unix milliseconds. */
    verifiedAt: schema.number(),
    /** The exclusive expiry, in Unix milliseconds. */
    expiresAt: schema.number(),
    /** The represented identity. */
    subject: Subject,
    /** How strongly and how recently the subject authenticated. */
    assurance: AuthenticationAssurance.optional(),
    /** The identifiers the subject proved control of. */
    identifiers: schema.array(VerifiedIdentifier).optional(),
    /** The verified principals and the subject sets the caller belongs to. */
    subjects: schema.array(Subject),
    /** The acting principals in order, the last sending the request. */
    delegates: schema.array(Delegate).optional(),
    /** The verified deployments of workload identities. */
    deployments: schema.array(CallerDeployment).optional(),
    /** The trusted attributes access policies read. */
    attributes: schema.record(schema.string(), Attribute).optional(),
    /** A lending of the caller's authority to the installation it called, which its holder signed, for the calls that installation sends. */
    delegation: schema.string().min(1).optional(),
});

/** The header carrying the caller a host forwards to a runner. */
export const CALLER_HEADER = "x-destack-caller";

/** A verified caller. */
export class Caller<Credential extends CredentialReference = CredentialReference> {
    /** The host-verified authentication. */
    readonly authentication: CallerAuthentication<Credential>;

    /** Create the caller from its verified authentication. */
    constructor(authentication: CallerAuthentication<Credential>) {
        this.authentication = structuredClone(authentication);
    }

    /** Read the caller a host forwarded with a request, or null for a request without one. */
    static forwarded(request: Request): Caller | null {
        const forwarded = request.headers.get(CALLER_HEADER);

        return forwarded === null
            ? null
            : new Caller(CallerAuthentication.parse(JSON.parse(forwarded)));
    }

    /** Forward the caller to a runner in a request's headers. */
    forward(headers: Headers): void {
        headers.set(CALLER_HEADER, JSON.stringify(this.authentication));
    }

    /** The credential reference. */
    get credential(): Credential {
        return this.authentication.credential;
    }

    /** The time the caller's authentication lapses, in Unix milliseconds. */
    get lapsesAt(): number {
        const { expiresAt, verifiedAt } = this.authentication;

        return Math.min(expiresAt, verifiedAt + CALLER_LIFETIME_MILLISECONDS);
    }

    /** Hold a time within the caller's lifetime: the time itself, or just before the lapse once passed. */
    within(now: number): number {
        return Math.min(now, this.lapsesAt - 1);
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
            now >= this.lapsesAt
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
            session: SessionKey.of(authentication.credential),
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

        // allow a space key to assert only installations of its space, calling any space
        if (
            authority.kind === "space" &&
            (authentication.identifiers?.length ||
                authentication.delegates?.length ||
                subjects.some(
                    (subject) =>
                        !principal.installation.is(subject) || subject.scope !== authority.spaceId,
                ))
        ) {
            throw new ServiceError("UNAUTHORIZED", { message: "token exceeds issuer authority" });
        }

        // bind each asserted installation to one deployment
        const workloads = [
            ...subjects,
            ...(authentication.delegates ?? []).map((delegate) => delegate.subject),
        ].filter((subject) => principal.installation.is(subject));
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

/** A verified caller in transit with a credential reference from its verifier. */
export type CallerAuthentication<Credential extends CredentialReference = CredentialReference> =
    Omit<schema.Infer<typeof CallerAuthentication>, "credential"> & {
        /** The credential reference. */
        readonly credential: Credential;
    };

/** A credential a caller presented, by kind and identifier. */
export interface CredentialReference {
    /** The credential kind, such as session or host-key. */
    readonly kind: string;
    /** The credential identifier. */
    readonly id: string;
}

/** A caller's session key in access decisions: its credential's kind and identifier. */
export const SessionKey = {
    /** Build a credential's session key. */
    of(credential: CredentialReference): string {
        return `${credential.kind}:${credential.id}`;
    },

    /** Read the credential identifier of a session key of one kind, absent for another kind. */
    id(key: string | undefined, kind: string): string | undefined {
        const prefix = `${kind}:`;

        return key?.startsWith(prefix) === true ? key.slice(prefix.length) : undefined;
    },
};

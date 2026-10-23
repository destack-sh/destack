import { type Identifier, Instant, JsonValue, nth, schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import { Scope, Subject, Watermark } from "@destack/sync";
import type { Scalar } from "@destack/db";
import { AccessError } from "../error/index.ts";
import { ANONYMOUS, principal } from "../declare/principal.ts";
import { Attribute } from "../declare/expression.ts";

/** The assurance of a principal proven by its server's key: phishing-resistant, as level 3 is. */
export const KEY_ASSURANCE = 3;

/** The largest clock difference between a credential's verifier and the server checking it. */
export const CLOCK_TOLERANCE_MILLISECONDS = 5000;

/** The header that forwards a verified caller from a host to its runner. */
export const CALLER_HEADER = "Destack-Caller";

/** The schema of a contact. */
const contactSchema = schema.object({
    /** The medium the address belongs to. */
    medium: schema.literal("email"),
    /** The address on the medium, in lowercase. */
    address: schema
        .email()
        .max(320)
        .regex(/^[^A-Z]+$/u),
});
/** A way to contact someone outside Destack that they prove control of. */
export type Contact = schema.Infer<typeof contactSchema>;

/** A way to contact someone outside Destack, with proven control, after Matrix's 3PIDs. */
export const Contact = Object.assign(contactSchema, {
    /** Read an email address as the contact its owner proves. */
    email(address: string): Contact {
        return contactSchema.parse({ medium: "email", address: address.trim().toLowerCase() });
    },

    /** Key a contact as its principal's identifier, such as `email:carol@example.com`. */
    key(contact: Contact): string {
        return `${contact.medium}:${contact.address}`;
    },

    /** Reference a contact as a principal for relationships and invitations. */
    subject(contact: Contact): Subject {
        return principal.contact.reference(Scope.universe.id, Contact.key(contact));
    },
});

/** How strongly and how recently a caller authenticated. */
export const AuthenticationAssurance = schema.object({
    /** The assurance level: 1 for one factor, 2 for several, 3 for phishing-resistant ones. */
    level: schema.number().int().min(1).max(3),
    /** The authentication time in UTC epoch milliseconds. */
    authenticatedAt: Instant,
});
/** How strongly and how recently a caller authenticated. */
export type AuthenticationAssurance = schema.Infer<typeof AuthenticationAssurance>;

/** A verified caller: who the call is for, who acts for them, and what they belong to. */
export interface Caller {
    /** Who the call is for: a principal, the anonymous one for a caller proving no identity. */
    readonly subject: Subject;
    /** The principals acting for the subject in order, the last sending the call (RFC 8693 act). */
    readonly actors: readonly Subject[];
    /** The subject sets the authenticator vouches the subject is in, such as account members. */
    readonly groups: readonly Subject[];
    /** The contacts, such as an email address, the subject proved control of. */
    readonly contacts: readonly Contact[];
    /** The grants each actor acts under, from the one before it, none for an impersonation. */
    readonly grants: readonly GrantClaim[];
}

/** The grant a delegated caller acts under: its identifier and the scope storing it. */
export const GrantClaim = schema.object({
    /** The grant's identifier. */
    id: schema.identifier("grant"),
    /** The scope storing the grant: its delegator's. */
    scope: schema.string().min(1),
});
/** The grant a delegated caller acts under. */
export type GrantClaim = schema.Infer<typeof GrantClaim>;

/** The schema of the credential a request presented. */
const credentialSchema = schema.object({
    /** The credential kind, such as session. */
    kind: schema.string().min(1),
    /** The credential identifier. */
    id: schema.string().min(1),
    /** The client whose key the credential is bound to, absent for an unbound one. */
    client: schema.identifier("client").exactOptional(),
    /** The package the credential was verified for. */
    audience: PackageId,
    /** The credential's scope with the scopes inside it, absent for an unscoped one. */
    scope: schema.string().min(1).exactOptional(),
    /** The verification time in UTC epoch milliseconds. */
    verifiedAt: Instant,
    /** The time the credential lapses: its expiry, capped by its audience's lifetime. */
    expiresAt: Instant,
    /** The access token's identifier (RFC 7519 jti), absent for one presented directly. */
    tokenId: schema.string().min(1).exactOptional(),
    /** How strongly and how recently its subject authenticated (OIDC acr, auth_time). */
    assurance: AuthenticationAssurance.exactOptional(),
    /** The deployments the workload identities among the subject and actors run in. */
    deployments: schema
        .array(
            schema.object({
                /** The workload identity. */
                subject: Subject,
                /** The deployment. */
                id: schema.identifier("deployment"),
            }),
        )
        .exactOptional(),
    /** Trusted attributes access policies read, such as IAM's condition keys. */
    attributes: schema.record(schema.string(), Attribute).exactOptional(),
    /** The log positions its grants were written at, which decisions under it wait for. */
    watermarks: schema.array(Watermark).min(1).exactOptional(),
});
/** The credential a request presented. */
export type Credential = schema.Infer<typeof credentialSchema>;

/** The credential a request presented, verified for one audience until it lapses (RFC 9068). */
export const Credential = Object.assign(credentialSchema, {
    /** Key a credential as `<kind>:<id>`, for credential conditions. */
    key(credential: Pick<Credential, "kind" | "id">): string {
        return `${credential.kind}:${credential.id}`;
    },

    /** Read the deployment a credential vouches a workload identity runs in, if any. */
    deployment(credential: Credential, subject: Subject): Identifier<"deployment"> | undefined {
        const deployments = credential.deployments ?? [];

        return deployments.find((entry) => Subject.same(entry.subject, subject))?.id;
    },

    /** Require a credential verified and unlapsed at a time. */
    requireLive(credential: Credential, now: number): void {
        if (
            credential.verifiedAt > now + CLOCK_TOLERANCE_MILLISECONDS ||
            now >= credential.expiresAt
        ) {
            throw new AccessError("AUTHENTICATION_EXPIRED", "caller authentication is not current");
        }
    },

    /** Require a credential current at a time for an audience and scope. */
    requireCurrent(credential: Credential, audience: PackageId, now: number, scope?: string) {
        // refuse a credential for another audience or scope
        if (
            credential.audience !== audience ||
            (credential.scope !== undefined && credential.scope !== scope)
        ) {
            throw new AccessError(
                "AUDIENCE_MISMATCH",
                "caller authentication has a different audience or scope",
            );
        }

        // refuse a lapsed credential
        Credential.requireLive(credential, now);
    },

    /** Clamp a time to a credential's lifetime: the time, or just before it lapses. */
    clamp(credential: Credential, now: number): number {
        return Math.min(now, credential.expiresAt - 1);
    },
});

/** The schema of a verified caller. */
const callerSchema = schema.object({
    /** Who the call is for. */
    subject: Subject,
    /** The principals acting for the subject in order, the last sending the call. */
    actors: schema.array(Subject),
    /** The subject sets the authenticator vouches the subject is in. */
    groups: schema.array(Subject),
    /** The contacts the subject proved control of. */
    contacts: schema.array(contactSchema),
    /** The grants each actor acts under, from the one before it. */
    grants: schema.array(GrantClaim),
});

/** A verified caller with the credential it presented, as a verifier returns them. */
export interface Authentication {
    /** The verified caller. */
    readonly caller: Caller;
    /** The credential it presented. */
    readonly credential: Credential;
}

/** The schema of a verified caller with the credential it presented. */
export const Authentication = schema.object({
    /** The verified caller. */
    caller: callerSchema,
    /** The credential it presented. */
    credential: credentialSchema,
});

/** A verified caller: its schema, and the principals and contacts it derives. */
export const Caller = {
    /** The schema of a verified caller. */
    schema: callerSchema,

    /** The caller of a principal running its code, proven by its server's key. */
    of(subject: Subject): Caller {
        return { subject, actors: [], groups: [], contacts: [], grants: [] };
    },

    /** The caller who proves no identity. */
    anonymous(): Caller {
        return Caller.of(ANONYMOUS);
    },

    /** Report whether a caller proves no identity. */
    isAnonymous(caller: Pick<Caller, "subject">): boolean {
        return principal.anonymous.is(caller.subject);
    },

    /** Key a caller by subject and sending principal, for retries and writers. */
    key(caller: Pick<Caller, "subject" | "actors">): string {
        return JSON.stringify([Subject.key(caller.subject), Subject.key(Caller.sender(caller))]);
    },

    /** Read the principal sending the call: the last actor, else the subject. */
    sender(caller: Pick<Caller, "subject" | "actors">): Subject {
        return caller.actors.at(-1) ?? caller.subject;
    },

    /** Read the principal whose authority decides: the sender under grants, else the subject. */
    principal(caller: Pick<Caller, "subject" | "actors" | "grants">): Subject {
        return caller.grants.length === 0 ? caller.subject : Caller.sender(caller);
    },

    /** Report whether a caller's actors act for its subject under grants. */
    isDelegated(caller: Pick<Caller, "actors" | "grants">): boolean {
        return caller.grants.length > 0 && caller.actors.length > 0;
    },

    /** Read each grant of a chain with its delegator, the actor before it, and its delegate. */
    links(caller: Pick<Caller, "subject" | "actors" | "grants">): {
        readonly grant: GrantClaim;
        readonly delegator: Subject;
        readonly delegate: Subject;
    }[] {
        return caller.grants.map((grant, index) => ({
            grant,
            delegator: index === 0 ? caller.subject : nth(caller.actors, index - 1),
            delegate: nth(caller.actors, index),
        }));
    },

    /** Require a grant per actor or none, and actors to act for an identified subject. */
    requireChain(caller: Caller): void {
        if (caller.grants.length > 0 && caller.grants.length !== caller.actors.length) {
            throw new AccessError(
                "INVALID_DELEGATION",
                "a delegation chain needs a grant per actor",
            );
        } else if (caller.actors.length > 0 && Caller.isAnonymous(caller)) {
            throw new AccessError("INVALID_DELEGATION", "delegation chain is inconsistent");
        }
    },

    /** Read the contacts the subject proved, none for actors under a grant. */
    contacts(caller: Caller): readonly Contact[] {
        return Caller.isDelegated(caller) ? [] : caller.contacts;
    },

    /** Read the caller and credential a host forwarded with a request, absent without them. */
    forwarded(request: Request): Authentication | undefined {
        // read nothing without the header
        const forwarded = request.headers.get(CALLER_HEADER);
        if (forwarded === null) {
            return undefined;
        }

        // refuse a header that is not a JSON caller with its credential
        const parsed = Authentication.safeParse(JsonValue.parse(forwarded));
        if (!parsed.success) {
            throw new AccessError("INVALID_FORWARDED_CALLER", "the forwarded caller is malformed");
        }

        return parsed.data;
    },

    /** Forward a caller and its credential to a runner in a request's headers. */
    forward(authentication: Authentication, headers: Headers): void {
        headers.set(CALLER_HEADER, JSON.stringify(authentication));
    },
};

/** What the authorizer decides on: a verified caller with what its request presented, at a time. */
export interface AccessContext extends Caller {
    /** The credential the request presented, absent for a principal running its code. */
    readonly credential?: Credential;
    /** The digests of the link secrets the request presented. */
    readonly linkSecrets: readonly string[];
    /** The request context the policies' conditions read. */
    readonly attributes: Readonly<Record<string, Attribute>>;
    /** The request's identifier, for conditions bound to one request. */
    readonly request?: string;
    /** The approved change the call applies, which lets it use approval-mode grant permissions. */
    readonly approval?: string;
    /** Trusted time of the decision in UTC epoch milliseconds. */
    readonly now: number;
}

/** Build access contexts and read their request attributes. */
export const AccessContext = {
    /** Decide for a caller at a time, with what its request presented. */
    of(
        caller: Caller,
        now: number,
        presented: {
            /** The credential the request presented, absent for a principal's code. */
            readonly credential?: Credential | undefined;
            /** The digests of the link secrets the request presented. */
            readonly linkSecrets?: readonly string[];
            /** The request's identifier. */
            readonly request?: string | undefined;
            /** The approved change the call applies. */
            readonly approval?: string | undefined;
        } = {},
    ): AccessContext {
        const { credential, linkSecrets = [], request, approval } = presented;

        return {
            ...caller,
            ...(credential === undefined ? {} : { credential }),
            ...(approval === undefined ? {} : { approval }),
            linkSecrets,
            attributes: credential?.attributes ?? {},
            now,
            ...(request === undefined ? {} : { request }),
        };
    },

    /** Read how strongly the caller authenticated, a key's strength for a principal's code. */
    assurance(
        context: Pick<AccessContext, "subject" | "credential" | "now">,
    ): AuthenticationAssurance | undefined {
        return context.credential !== undefined
            ? context.credential.assurance
            : Caller.isAnonymous(context)
              ? undefined
              : { level: KEY_ASSURANCE, authenticatedAt: context.now };
    },

    /** Report whether a context's caller is remote, unlike a principal running its code. */
    isRemote(context: Pick<AccessContext, "subject" | "credential">): boolean {
        return context.credential !== undefined || Caller.isAnonymous(context);
    },

    /** Read a request attribute for a policy condition and refuse a missing or non-finite one. */
    attribute(context: AccessContext, name: string): Scalar {
        const value = Object.hasOwn(context.attributes, name)
            ? context.attributes[name]
            : undefined;
        if (value === undefined || (typeof value === "number" && !Number.isFinite(value))) {
            throw new AccessError(
                "INVALID_ATTRIBUTE",
                `missing or invalid context attribute: ${name}`,
            );
        }

        return value;
    },
};

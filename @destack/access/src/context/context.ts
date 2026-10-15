import { defineSchema, type Identifier, Instant, JsonValue, schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import { Scope, Subject } from "@destack/sync";
import type { Scalar } from "@destack/db";
import { AccessError } from "../error/index.ts";
import { isPrincipal, principal } from "../declare/principal.ts";
import { GrantPermission } from "../grant/permission.ts";
import { Attribute } from "../declare/expression.ts";

/** The assurance of a principal proven by its server's key: phishing-resistant, as level 3 is. */
export const KEY_ASSURANCE = 3;

/** The largest clock difference between a credential's verifier and the server checking it. */
export const CLOCK_TOLERANCE_MILLISECONDS = 5000;

/** The header that forwards a verified caller from a host to its runner. */
export const CALLER_HEADER = "Destack-Caller";

/** The schema of a contact. */
const contactSchema = defineSchema(
    schema.object({
        /** The medium the address belongs to. */
        medium: schema.literal("email"),
        /** The address on the medium, in lowercase. */
        address: schema
            .email()
            .max(320)
            .regex(/^[^A-Z]+$/u),
    }),
);
/** A way to reach someone outside Destack that they prove control of. */
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
export const AuthenticationAssurance = defineSchema(
    schema.object({
        /** The assurance level: 1 for one factor, 2 for several, 3 for phishing-resistant ones. */
        level: schema.number().int().min(1).max(3),
        /** The authentication time in UTC epoch milliseconds. */
        authenticatedAt: Instant,
    }),
);
/** How strongly and how recently a caller authenticated. */
export type AuthenticationAssurance = schema.Infer<typeof AuthenticationAssurance>;

/** A verified caller: the represented subject, the principals acting for it, and what it proved. */
export interface Caller {
    /** The represented subject, required for delegated calls. */
    readonly subject?: Subject;
    /** The principals acting for the subject in order, the last sending the call (RFC 8693 act). */
    readonly delegates?: readonly Subject[];
    /** The grant limiting the call to what the subject gave, absent for its whole authority. */
    readonly grant?: CallerGrant;
    /** The verified principals and subject sets, empty for an anonymous caller. */
    readonly subjects: readonly Subject[];
    /** How strongly and how recently the caller authenticated. */
    readonly assurance?: AuthenticationAssurance;
    /** The contacts, such as an email address, the caller proved control of. */
    readonly contacts?: readonly Contact[];
    /** The procedure paths of the audience the space granted the installation. */
    readonly calls?: readonly string[];
    /** Digests of the link secrets the caller presented. */
    readonly linkSecrets?: readonly string[];
    /** Trusted attributes access policies read. */
    readonly attributes?: Readonly<Record<string, Attribute>>;
    /** The caller's presented credential, absent if anonymous or running its own code. */
    readonly credential?: CallerCredential;
    /** The deployments the workload identities among the subjects run in. */
    readonly deployments?: readonly CallerDeployment[];
}

/** A caller with a represented subject, as every mutation's caller has. */
export type SignedCaller = Caller & { readonly subject: Subject };

/** A caller's credential, verified for one audience until it lapses (RFC 7662). */
export const CallerCredential = schema.object({
    /** The credential kind, such as session. */
    kind: schema.string().min(1),
    /** The credential identifier. */
    id: schema.string().min(1),
    /** The client whose key the credential is bound to, absent for an unbound credential. */
    client: schema.identifier("client").exactOptional(),
    /** The package the credential was verified for. */
    audience: PackageId,
    /** The credential's scope with the scopes inside it, absent for an unscoped credential. */
    scope: schema.string().min(1).exactOptional(),
    /** The verification time in UTC epoch milliseconds. */
    verifiedAt: Instant,
    /** The time the credential lapses: its expiry, capped by its audience's lifetime. */
    expiresAt: Instant,
    /** The credential's access token identifier (RFC 7519 jti), absent if presented directly. */
    token: schema.string().min(1).exactOptional(),
});
/** A credential a caller presented. */
export type CallerCredential = schema.Infer<typeof CallerCredential>;

/** A workload identity authenticated within one deployment. */
export const CallerDeployment = schema.object({
    /** The workload identity. */
    subject: Subject,
    /** The deployment. */
    id: schema.identifier("deployment"),
});
/** A workload identity authenticated within one deployment. */
export type CallerDeployment = schema.Infer<typeof CallerDeployment>;

/** The grant a caller acts under: its identifier, the scope keeping it, and what it gives. */
export const CallerGrant = schema.object({
    /** The grant's identifier. */
    id: schema.identifier("grant"),
    /** The scope keeping the grant: its delegator's. */
    scope: schema.string().min(1),
    /** The permissions the grant gives (RFC 9396 authorization details). */
    permissions: schema.array(GrantPermission.schema),
});
/** The grant a caller acts under. */
export type CallerGrant = schema.Infer<typeof CallerGrant>;

/** A verified caller: its schema, and the principals and contacts it derives. */
export const Caller = {
    /** The schema of a verified caller. */
    schema: schema.object({
        /** The represented subject, required for delegated calls. */
        subject: Subject.exactOptional(),
        /** The principals acting for the subject in order, the last sending the call. */
        delegates: schema.array(Subject).exactOptional(),
        /** The grant limiting the call to what the subject gave. */
        grant: CallerGrant.exactOptional(),
        /** The verified principals and subject sets, empty for an anonymous caller. */
        subjects: schema.array(Subject),
        /** How strongly and how recently the caller authenticated. */
        assurance: AuthenticationAssurance.exactOptional(),
        /** The contacts the caller proved control of. */
        contacts: schema.array(Contact).exactOptional(),
        /** The procedures of the audience an installation's space granted it. */
        calls: schema.array(schema.string().min(1)).exactOptional(),
        /** Digests of the link secrets the caller presented. */
        linkSecrets: schema.array(schema.string().min(1)).exactOptional(),
        /** Trusted attributes access policies read. */
        attributes: schema.record(schema.string(), Attribute).exactOptional(),
        /** The credential the caller presented. */
        credential: CallerCredential.exactOptional(),
        /** The deployments the workload identities among the subjects run in. */
        deployments: schema.array(CallerDeployment).exactOptional(),
    }),

    /** Key a caller by represented subject and sending principal, for retries and writers. */
    key(caller: Pick<Caller, "subject" | "delegates">): string {
        const { subject } = caller;
        const sender = Caller.sender(caller);

        return JSON.stringify([
            subject === undefined ? null : Subject.key(subject),
            sender === undefined ? null : Subject.key(sender),
        ]);
    },

    /** Report whether a caller is remote, unlike a principal running its own code. */
    isRemote(caller: Pick<Caller, "subject" | "credential">): boolean {
        return caller.credential !== undefined || caller.subject === undefined;
    },

    /** The caller of a principal running its own code at a time, proven by its server's key. */
    of(subject: Subject, now: number): Caller {
        return {
            subject,
            subjects: [subject],
            assurance: { level: KEY_ASSURANCE, authenticatedAt: now },
        };
    },

    /** Read the principal acting for the caller: the sender under a grant, else the subject. */
    principal(caller: Caller): Subject | undefined {
        const acting = Caller.isDelegated(caller) ? caller.delegates?.at(-1) : undefined;

        return acting ?? caller.subject ?? caller.subjects.find(isPrincipal);
    },

    /** Require the principal acting for the caller. */
    requirePrincipal(caller: Caller): Subject {
        const acting = Caller.principal(caller);
        if (acting === undefined) {
            throw new AccessError("FORBIDDEN", "the request needs an authenticated principal");
        }

        return acting;
    },

    /** Report whether a caller represents a subject, as a mutation's caller must. */
    isSigned(caller: Caller): caller is SignedCaller {
        return caller.subject !== undefined;
    },

    /** Read the principal sending the call: the last delegate, else the subject. */
    sender(caller: Pick<Caller, "subject" | "delegates">): Subject | undefined {
        return caller.delegates?.at(-1) ?? caller.subject;
    },

    /** Require the principal sending the call, refusing an anonymous caller. */
    requireSender(caller: Pick<Caller, "subject" | "delegates">): Subject {
        const sender = Caller.sender(caller);
        if (sender === undefined) {
            throw new AccessError("UNAUTHORIZED", "the request needs an authenticated sender");
        }

        return sender;
    },

    /** Report whether a caller's delegates act for its subject under a grant. */
    isDelegated(caller: Pick<Caller, "delegates" | "grant">): boolean {
        return caller.grant !== undefined && (caller.delegates ?? []).length > 0;
    },

    /** Require a caller's delegates to act for one of its verified subjects. */
    requireChain(caller: Caller): void {
        // refuse a grant without delegates
        const represented = caller.subject;
        if (caller.grant !== undefined && (caller.delegates ?? []).length === 0) {
            throw new AccessError("INVALID_CONTEXT", "a grant needs delegates");
        }

        // refuse delegates acting for a subject the caller did not verify
        if (
            (caller.delegates ?? []).length > 0 &&
            (represented === undefined ||
                !caller.subjects.some((subject) => Subject.same(subject, represented)))
        ) {
            throw new AccessError("INVALID_CONTEXT", "delegation chain is inconsistent");
        }
    },

    /** Read the contacts the represented subject proved, none for delegates under a grant. */
    contacts(caller: Caller): readonly Contact[] {
        return Caller.isDelegated(caller) ? [] : (caller.contacts ?? []);
    },

    /** Read a caller's credential key, `<kind>:<id>`, for credential conditions. */
    credentialKey(caller: Pick<Caller, "credential">): string | undefined {
        const { credential } = caller;

        return credential === undefined ? undefined : `${credential.kind}:${credential.id}`;
    },

    /** Read the deployment of a workload identity among the caller's subjects, if any. */
    deployment(caller: Caller, subject: Subject): Identifier<"deployment"> | undefined {
        const deployments = caller.deployments ?? [];

        return deployments.find((entry) => Subject.same(entry.subject, subject))?.id;
    },

    /** Require the credential a caller presented, refusing a caller without one. */
    requireCredential(caller: Caller): CallerCredential {
        if (caller.credential === undefined) {
            throw new AccessError("UNAUTHORIZED", "missing caller credential");
        }

        return caller.credential;
    },

    /** Require a caller's credential verified and unlapsed at a time. */
    requireLive(caller: Caller, now: number): void {
        const { credential } = caller;
        if (
            credential !== undefined &&
            (credential.verifiedAt > now + CLOCK_TOLERANCE_MILLISECONDS ||
                now >= credential.expiresAt)
        ) {
            throw new AccessError("UNAUTHORIZED", "caller authentication is not current");
        }
    },

    /** Require a caller current at a time for an audience and scope, with a verified subject. */
    requireCurrent(caller: Caller, audience: PackageId, now: number, scope?: string): void {
        // refuse a credential for another audience or scope
        const { credential, subject } = caller;
        if (
            credential !== undefined &&
            (credential.audience !== audience ||
                (credential.scope !== undefined && credential.scope !== scope))
        ) {
            throw new AccessError(
                "UNAUTHORIZED",
                "caller authentication has a different audience or scope",
            );
        }

        // refuse a lapsed credential and a subject outside the verified ones
        Caller.requireLive(caller, now);
        if (subject !== undefined && !caller.subjects.some((each) => Subject.same(each, subject))) {
            throw new AccessError(
                "UNAUTHORIZED",
                "caller subject is missing from verified identities",
            );
        }
    },

    /** Clamp a time to a caller's lifetime: the time, or just before its credential lapses. */
    clamp(caller: Caller, now: number): number {
        return caller.credential === undefined
            ? now
            : Math.min(now, caller.credential.expiresAt - 1);
    },

    /** Read the caller a host forwarded with a request, absent for a request without one. */
    forwarded(request: Request): Caller | undefined {
        // read no caller without the header
        const forwarded = request.headers.get(CALLER_HEADER);
        if (forwarded === null) {
            return undefined;
        }

        // refuse a header that is not a JSON caller
        const parsed = Caller.schema.safeParse(JsonValue.parse(forwarded));
        if (!parsed.success) {
            throw new AccessError("INVALID_CONTEXT", "the forwarded caller is malformed");
        }

        return parsed.data;
    },

    /** Forward a caller to a runner in a request's headers. */
    forward(caller: Caller, headers: Headers): void {
        headers.set(CALLER_HEADER, JSON.stringify(caller));
    },
};

/** What the authorizer decides on: a verified caller at a time, within a request. */
export interface AccessContext extends Caller {
    /** The request's identifier, for conditions bound to one request. */
    readonly request?: string;
    /** Trusted time of the decision in UTC epoch milliseconds. */
    readonly now: number;
    /** Trusted attributes referenced by declarations. */
    readonly attributes: Readonly<Record<string, Attribute>>;
}

/** Build access contexts and read their request attributes. */
export const AccessContext = {
    /** Decide for a caller at a time, within a request when given. */
    of(caller: Caller, now: number, request?: string): AccessContext {
        return {
            ...caller,
            attributes: caller.attributes ?? {},
            now,
            ...(request === undefined ? {} : { request }),
        };
    },

    /** Read a request attribute for a policy condition and refuse a missing or non-finite one. */
    attribute(context: AccessContext, name: string): Scalar {
        const value = Object.hasOwn(context.attributes, name)
            ? context.attributes[name]
            : undefined;
        if (value === undefined || (typeof value === "number" && !Number.isFinite(value))) {
            throw new AccessError(
                "INVALID_CONTEXT",
                `missing or invalid context attribute: ${name}`,
            );
        }

        return value;
    },
};

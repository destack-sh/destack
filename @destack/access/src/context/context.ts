import { defineSchema, Instant, schema } from "@destack/schema";
import { Scope, Subject } from "@destack/sync";
import type { Scalar } from "@destack/db";
import { AccessError } from "../error/index.ts";
import { isPrincipal, principal } from "../declare/principal.ts";
import { Restriction } from "./restriction.ts";
import { Attribute } from "../declare/expression.ts";


/** The assurance of a principal proven by its server's key: phishing-resistant, as level 3 is. */
export const KEY_ASSURANCE = 3;
/** The schema of a contact. */
const contactSchema = defineSchema(
    schema.object({
        /** The medium the address belongs to. */
        medium: schema.literal("email"),
        /** The address on the medium, in lowercase. */
        address: schema
            .email()
            .max(320)
            .regex(/^[^A-Z]+$(?![\s\S])/u),
    }),
);
/** A way to reach someone outside Destack that they prove control of. */
export type Contact = schema.Infer<typeof contactSchema>;

/** A way to reach someone outside Destack that they prove control of, after Matrix's third-party identifiers. */
export const Contact = Object.assign(contactSchema, {
    /** Read an email address as the contact its owner proves. */
    email(address: string): Contact {
        return contactSchema.parse({ medium: "email", address: address.trim().toLowerCase() });
    },

    /** Key a contact as its principal's identifier, such as `email:carol@example.com`. */
    key(contact: Contact): string {
        return `${contact.medium}:${contact.address}`;
    },

    /** Reference a contact as the principal relationships and invitations name. */
    subject(contact: Contact): Subject {
        return principal.contact.reference(Scope.universe.id, Contact.key(contact));
    },
});

/** One principal acting in a request, for the principal before it in the chain, the first for the represented subject. */
export const Delegate = defineSchema(
    schema.object({
        /** The acting principal. */
        subject: Subject,
        /** The authority it acts with: lent by the one before, or all of the one before's authority. */
        authority: schema.enum(["lent", "full"]),
    }),
);
/** One principal acting in a request, for the principal before it in the chain, the first for the represented subject. */
export type Delegate = schema.Infer<typeof Delegate>;

/** How strongly and how recently a caller authenticated. */
export const AuthenticationAssurance = defineSchema(
    schema.object({
        /** The assurance level: 1 for one factor, 2 for several, 3 for phishing-resistant factors. */
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
    /** The principals acting for the subject in order, the last sending the call. */
    readonly delegates?: readonly Delegate[];
    /** The verified principals and subject sets, empty for an anonymous caller. */
    readonly subjects: readonly Subject[];
    /** How strongly and how recently the caller authenticated. */
    readonly assurance?: AuthenticationAssurance;
    /** The contacts, such as an email address, the caller proved control of. */
    readonly contacts?: readonly Contact[];
    /** The permissions a restricted credential allows, intersected with every grant. */
    readonly permissions?: readonly Restriction[];
    /** The session or agent instance the caller belongs to, matched by grants bound to one session. */
    readonly session?: string;
    /** Digests of the link secrets the caller presented. */
    readonly linkSecrets?: readonly string[];
    /** Trusted attributes access policies read. */
    readonly attributes?: Readonly<Record<string, Attribute>>;
    /** The token a space exchanged for the caller, lending its authority to one installation (RFC 8693). */
    readonly delegation?: string;
    /** The one scope a scoped credential acts in, absent for a credential acting anywhere. */
    readonly scope?: string;
    /** The credential the caller presented, absent for an anonymous caller or a principal running its own code. */
    readonly credential?: CallerCredential;
    /** The deployments the workload identities among the subjects run in. */
    readonly deployments?: readonly CallerDeployment[];
    /** The Loan the caller's authority came with, absent for any other caller. */
    readonly loan?: CallerLoan;
}

/** A credential a caller presented: its kind, its identifier and the client whose key it is bound to. */
export const CallerCredential = schema.object({
    /** The credential kind. */
    kind: schema.string().min(1),
    /** The credential identifier. */
    id: schema.string().min(1),
    /** The client whose key the credential is bound to, absent for an unbound credential. */
    client: schema.identifier("client").exactOptional(),
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

/** The Loan a caller's authority came with: its identifier, the home space that signed it, and the one method it lends. */
export const CallerLoan = schema.object({
    /** The Loan's identifier (RFC 7519 4.1.7). */
    id: schema.string().min(1),
    /** The home space that signed it. */
    issuer: schema.string().min(1),
    /** The one method it lends, as `<package>/<object>.<method>`. */
    method: schema.string().min(1),
});
/** The Loan a caller's authority came with. */
export type CallerLoan = schema.Infer<typeof CallerLoan>;

/** A verified caller: its schema, and the principals and contacts it derives. */
export const Caller = {
    /** The schema of a verified caller. */
    schema: schema.object({
        /** The represented subject, required for delegated calls. */
        subject: Subject.exactOptional(),
        /** The principals acting for the subject in order, the last sending the call. */
        delegates: schema.array(Delegate).exactOptional(),
        /** The verified principals and subject sets, empty for an anonymous caller. */
        subjects: schema.array(Subject),
        /** How strongly and how recently the caller authenticated. */
        assurance: AuthenticationAssurance.exactOptional(),
        /** The contacts the caller proved control of. */
        contacts: schema.array(Contact).exactOptional(),
        /** The permissions a restricted credential allows. */
        permissions: schema.array(Restriction.schema).exactOptional(),
        /** The session or agent instance the caller belongs to. */
        session: schema.string().min(1).exactOptional(),
        /** Digests of the link secrets the caller presented. */
        linkSecrets: schema.array(schema.string().min(1)).exactOptional(),
        /** Trusted attributes access policies read. */
        attributes: schema.record(schema.string(), Attribute).exactOptional(),
        /** The token a space exchanged for the caller, lending its authority to one installation (RFC 8693). */
        delegation: schema.string().min(1).exactOptional(),
        /** The one scope a scoped credential acts in. */
        scope: schema.string().min(1).exactOptional(),
        /** The credential the caller presented. */
        credential: CallerCredential.exactOptional(),
        /** The deployments the workload identities among the subjects run in. */
        deployments: schema.array(CallerDeployment).exactOptional(),
        /** The Loan the caller's authority came with. */
        loan: CallerLoan.exactOptional(),
    }),

    /** Key a caller by its represented subject and the principal sending its calls, as retries and writers are identified. */
    key(caller: Pick<Caller, "subject" | "delegates">): string {
        const { subject } = caller;
        const sender = Caller.sender(caller);

        return JSON.stringify([
            subject === undefined ? null : Subject.key(subject),
            sender === undefined ? null : Subject.key(sender),
        ]);
    },

    /** Report whether a caller is remote: it presented a credential or none, unlike a principal running its own code. */
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

    /** Read the principal acting for the caller. */
    principal(caller: Caller): Subject | undefined {
        const acting = caller.delegates?.findLast((delegate) => delegate.authority === "lent");

        return acting?.subject ?? caller.subject ?? caller.subjects.find(isPrincipal);
    },

    /** Require the principal acting for the caller. */
    requirePrincipal(caller: Caller): Subject {
        const acting = Caller.principal(caller);
        if (acting === undefined) {
            throw new AccessError("FORBIDDEN", "the request needs an authenticated principal");
        }

        return acting;
    },

    /** Read the principal sending the call: the last delegate, else the subject. */
    sender(caller: Pick<Caller, "subject" | "delegates">): Subject | undefined {
        return caller.delegates?.at(-1)?.subject ?? caller.subject;
    },

    /** Pair each delegate with the principal it acts for, from the represented subject to the principal sending the request. */
    delegation(
        caller: Caller,
    ): { delegate: Subject; delegator: Subject; authority: Delegate["authority"] }[] {
        // allow direct calls
        const delegates = caller.delegates ?? [];
        if (delegates.length === 0) {
            return [];
        }

        // require the represented subject among the authenticated subjects
        const represented = caller.subject;
        if (
            !represented ||
            !caller.subjects.some((subject) => Subject.same(subject, represented))
        ) {
            throw new AccessError("INVALID_CONTEXT", "delegation chain is inconsistent");
        }

        // pair each delegate with the principal it acts for, whom a full delegate acts as
        let delegator = represented;

        return delegates.map(({ subject, authority }) => {
            const link = { delegate: subject, delegator, authority };
            if (authority === "lent") {
                delegator = subject;
            }

            return link;
        });
    },

    /** Read the contacts the represented subject proved, none under lent authority. */
    contacts(caller: Caller): readonly Contact[] {
        const isLent = caller.delegates?.some((delegate) => delegate.authority === "lent") ?? false;

        return isLent ? [] : (caller.contacts ?? []);
    },
};

/** What the authorizer decides on: a verified caller at a time, within a request. */
export interface AccessContext extends Caller {
    /** The request's identifier, matched by grants bound to one request. */
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

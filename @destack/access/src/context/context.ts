import { defineSchema, Instant, schema } from "@destack/schema";
import { Scope, Subject } from "@destack/sync";
import type { Scalar } from "@destack/db";
import { AccessError } from "../error/index.ts";
import { isPrincipal, principal } from "../policy/principal.ts";
import { Restriction } from "./restriction.ts";
import type { Attribute } from "../policy/expression.ts";

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

    /** Reference a contact as the principal relationships and proposals name. */
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
}

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
    }),

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

/** What the authorizer decides on: a verified caller and its request's attributes. */
export interface AccessContext extends Caller {
    /** The request's identifier, matched by grants bound to one request. */
    readonly request?: string;
    /** The session or agent instance the request belongs to, matched by grants bound to one session. */
    readonly session?: string;
    /** Digests of the link secrets the request presented. */
    readonly linkSecrets?: readonly string[];
    /** Trusted request time in UTC epoch milliseconds. */
    readonly now: number;
    /** Trusted request attributes referenced by declarations. */
    readonly attributes: Readonly<Record<string, Attribute>>;
}

/** Read the request attributes of an access context. */
export const AccessContext = {
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

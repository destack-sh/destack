import { defineSchema, Instant, schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import type { Scalar } from "@destack/db/query";
import { AccessError } from "../error/index.ts";
import { isPrincipal } from "../policy/principal.ts";
import { Restriction } from "./restriction.ts";
import type { Attribute } from "../policy/expression.ts";

/** The schema of a verified identifier. */
const verifiedIdentifier = defineSchema(
    schema
        .string()
        .min(3)
        .max(320)
        .regex(/^[a-z][a-z0-9-]*:\S+$(?![\s\S])/),
);
/** An identifier a principal proves control of, such as `email:bob@acme.com`, written `scheme:value`. */
export type VerifiedIdentifier = schema.Infer<typeof verifiedIdentifier>;

/** An identifier a principal proves control of, such as `email:bob@acme.com`, written `scheme:value`. */
export const VerifiedIdentifier = Object.assign(verifiedIdentifier, {
    /** Write an email address as the identifier its verified owner controls. */
    fromEmail(address: string): VerifiedIdentifier {
        return verifiedIdentifier.parse(`email:${address.trim().toLowerCase()}`);
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
    /** The principals acting for the subject in order; the last sends the call. */
    readonly delegates?: readonly Delegate[];
    /** The verified principals and subject sets; empty means anonymous. */
    readonly subjects: readonly Subject[];
    /** How strongly and how recently the caller authenticated. */
    readonly assurance?: AuthenticationAssurance;
    /** Identifiers, such as `email:bob@acme.com`, the caller proved control of. */
    readonly identifiers?: readonly VerifiedIdentifier[];
    /** The permissions a restricted credential allows, intersected with every grant. */
    readonly permissions?: readonly Restriction[];
}

/** A verified caller: its schema, and the principals and identifiers it derives. */
export const Caller = {
    /** The schema of a verified caller. */
    schema: schema.object({
        /** The represented subject, required for delegated calls. */
        subject: Subject.optional(),
        /** The principals acting for the subject in order; the last sends the call. */
        delegates: schema.array(Delegate).optional(),
        /** The verified principals and subject sets; empty means anonymous. */
        subjects: schema.array(Subject),
        /** How strongly and how recently the caller authenticated. */
        assurance: AuthenticationAssurance.optional(),
        /** Identifiers the caller proved control of. */
        identifiers: schema.array(VerifiedIdentifier).optional(),
        /** The permissions a restricted credential allows. */
        permissions: schema.array(Restriction.schema).optional(),
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

    /** Read the identifiers the represented subject proved, empty under lent authority. */
    identifiers(caller: Caller): readonly string[] {
        const isLent = caller.delegates?.some((delegate) => delegate.authority === "lent") ?? false;

        return isLent ? [] : (caller.identifiers ?? []);
    },
};

/** What the authorizer decides on: a verified caller and the facts of its request. */
export interface AccessContext extends Caller {
    /** The request's identifier, matched by grants bound to one request. */
    readonly request?: string;
    /** The session or agent instance the request belongs to, matched by grants bound to one session. */
    readonly session?: string;
    /** Digests of the capabilities, such as link secrets, the request presented. */
    readonly capabilities?: readonly string[];
    /** Trusted request time in UTC epoch milliseconds. */
    readonly now: number;
    /** Trusted request attributes referenced by declarations. */
    readonly attributes: Readonly<Record<string, Attribute>>;
}

/** Read the request attributes of an access context. */
export const AccessContext = {
    /** Read a request attribute for a policy condition and refuse a missing or non-finite one. */
    attribute(context: AccessContext, name: string): Scalar {
        const value = context.attributes[name];
        if (
            !Object.hasOwn(context.attributes, name) ||
            (typeof value === "number" && !Number.isFinite(value))
        ) {
            throw new AccessError(
                "INVALID_CONTEXT",
                `missing or invalid context attribute: ${name}`,
            );
        }

        return value!;
    },
};

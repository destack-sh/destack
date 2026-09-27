import { defineSchema, schema } from "@destack/schema";
import { AccessError } from "../error/index.ts";
import { sameSubject, Subject } from "../policy/subject.ts";
import type { Restriction } from "./restriction.ts";
import type { Attribute } from "../policy/expression.ts";

/** The scope of rows outside every other scope, such as users and organisations. */
export const GLOBAL_SCOPE = "global";

/** How long strong authentication elevates a caller for sensitive permissions, in milliseconds. */
export const ELEVATION_MILLISECONDS = 15 * 60 * 1000;

/** An identifier a principal proves control of, such as `email:bob@acme.com`, written `scheme:value`. */
export const VerifiedIdentifier = defineSchema(
    schema
        .string()
        .min(3)
        .max(320)
        .regex(/^[a-z][a-z0-9-]*:\S+$(?![\s\S])/),
);
/** An identifier a principal proves control of, such as `email:bob@acme.com`, written `scheme:value`. */
export type VerifiedIdentifier = schema.Infer<typeof VerifiedIdentifier>;

/** One principal acting in a request, for the principal before it in the chain, the first for the represented subject. */
export const Delegate = defineSchema(
    schema.object({
        /** The acting principal. */
        subject: Subject,
        /** What it acts with: what the one before lent it, or the one before's whole authority, as when impersonating. */
        authority: schema.enum(["lent", "full"]),
    }),
);
/** One principal acting in a request, for the principal before it in the chain, the first for the represented subject. */
export type Delegate = schema.Infer<typeof Delegate>;

/** Write an email address as the identifier its verified owner controls. */
export function emailIdentifier(address: string): VerifiedIdentifier {
    return VerifiedIdentifier.parse(`email:${address.trim().toLowerCase()}`);
}

/** Verified identities and attributes supplied by the authoritative caller. */
export interface AccessContext {
    /** The permissions a restricted credential allows, intersected with every grant. */
    readonly permissions?: readonly Restriction[];
    /** Authenticated represented identity, required for delegated calls. */
    readonly subject?: Subject;
    /** The principals acting in order, each for the one before and the first for the subject; the last sends the request. */
    readonly delegates?: readonly Delegate[];
    /** Current identities and verified subject sets; empty means anonymous. */
    readonly subjects: readonly Subject[];
    /** The request's identifier, matched by grants bound to one request. */
    readonly request?: string;
    /** The session or agent instance the request belongs to, matched by grants bound to one session. */
    readonly session?: string;
    /** Digests of the capabilities, such as link secrets, the request presented. */
    readonly capabilities?: readonly string[];
    /** Identifiers, such as `email:bob@acme.com`, the authenticating authority verified the caller controls. */
    readonly identifiers?: readonly VerifiedIdentifier[];
    /** How strongly and how recently the caller authenticated. */
    readonly assurance?: AuthenticationAssurance;
    /** Trusted request time in UTC epoch milliseconds. */
    readonly now: number;
    /** Trusted request attributes referenced by declarations. */
    readonly attributes: Readonly<Record<string, Attribute>>;
}

/** How strongly and how recently a caller authenticated. */
export const AuthenticationAssurance = defineSchema(
    schema.object({
        /** The assurance level: 1 for one factor, 2 for several, 3 for phishing-resistant factors. */
        level: schema.number().int().min(1).max(3),
        /** The authentication time in UTC epoch milliseconds. */
        authenticatedAt: schema.number().int().nonnegative(),
    }),
);
/** How strongly and how recently a caller authenticated. */
export type AuthenticationAssurance = schema.Infer<typeof AuthenticationAssurance>;

/** Pair each delegate with the principal it acts for, from the represented subject to the principal sending the request. */
export function delegationChain(
    context: AccessContext,
): { delegate: Subject; delegator: Subject; authority: Delegate["authority"] }[] {
    // allow direct calls
    const delegates = context.delegates ?? [];
    if (delegates.length === 0) {
        return [];
    }

    // require the represented subject among the authenticated subjects
    const represented = context.subject;
    if (!represented || !context.subjects.some((subject) => sameSubject(subject, represented))) {
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
}

/** Determine whether a request authenticated strongly and recently enough for sensitive permissions. */
export function isElevated(context: AccessContext): boolean {
    const assurance = context.assurance;

    return (
        assurance !== undefined &&
        assurance.level >= 2 &&
        context.now - assurance.authenticatedAt <= ELEVATION_MILLISECONDS
    );
}

/** Read the identifiers the represented subject proved, which a delegate acting with lent authority never inherits. */
export function verifiedIdentifiers(context: AccessContext): readonly string[] {
    const isLent = context.delegates?.some((delegate) => delegate.authority === "lent") ?? false;

    return isLent ? [] : (context.identifiers ?? []);
}

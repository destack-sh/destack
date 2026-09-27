import type {} from "@destack/package/import-meta";
import { grants, none, relation, union } from "./expression.ts";
import { Policy } from "./policy.ts";
import { AccessError } from "../error/index.ts";
import type { Subject } from "./subject.ts";
import type { AccessContext } from "../context/context.ts";

/** The package declaring access's own types. */
const OWNER = import.meta.destack.package;

/** The identity of the package declaring access's own types. */
export const ACCESS_PACKAGE_ID = OWNER.id;

/** The kinds of principal that authenticate: people, machines and installed software. */
export const principal = {
    /** A person, identified globally, and the scope holding their own objects. */
    user: new Policy(OWNER, { name: "user", permissions: {}, scope: true }),
    /** A machine running Destack, and the scope of its local operations. */
    host: new Policy(OWNER, { name: "host", permissions: {}, scope: true }),
    /** A region of Destack's hosted platform, administering the spaces placed in it. */
    region: new Policy(OWNER, { name: "region", permissions: {} }),
    /** An application installed into an account or space: the principal of software, wherever it runs. */
    installation: new Policy(OWNER, { name: "installation", permissions: {} }),
};

/** Every caller, signed in or anonymous, related through its wildcard; links condition such grants on a capability. */
export const anyone = new Policy(OWNER, { name: "anyone", permissions: {} });

/** A named set of permissions, including the permissions of the roles it includes. */
export const role = new Policy(OWNER, {
    name: "role",
    relations: { includes: { subjects: ["role"] } },
    permissions: { create: none(), read: none(), update: none(), delete: none(), share: none() },
});

/** A relation or role binding between a subject and an object: its subject reads it, and so does whoever may grant on its object. */
export const relationship = new Policy(OWNER, {
    name: "relationship",
    relations: { subject: { subjects: [principal.user, principal.host, principal.installation] } },
    permissions: { read: union(relation("subject"), grants("object")) },
});

/** A proposed relationship awaiting acceptance: its proposer, addressee and lender read it, and so does whoever may grant on its object. */
export const proposal = new Policy(OWNER, {
    name: "proposal",
    relations: {
        proposer: { subjects: [principal.user, principal.host, principal.installation] },
        addressee: { subjects: [principal.user, principal.host, principal.installation] },
        lender: { subjects: [principal.user, principal.host, principal.installation] },
    },
    permissions: {
        read: union(
            relation("proposer"),
            relation("addressee"),
            relation("lender"),
            grants("object"),
        ),
    },
});

/** The policies of access's own types, which every access model contains. */
export const INTRINSIC_POLICIES: readonly Policy[] = [
    principal.user,
    principal.host,
    principal.region,
    principal.installation,
    anyone,
    role,
    relationship,
    proposal,
];

/** Determine whether a subject is one principal rather than a set or another object. */
export function isPrincipal(subject: Subject): boolean {
    return Object.values(principal).some((kind) => kind.is(subject));
}

/** Read the principal acting in a request: the last delegate acting with lent authority, the represented subject, or else the authenticated principal. */
export function principalOf(context: AccessContext): Subject | undefined {
    const acting = context.delegates?.findLast((delegate) => delegate.authority === "lent");

    return acting?.subject ?? context.subject ?? context.subjects.find(isPrincipal);
}

/** Require the principal acting in a request. */
export function requirePrincipal(context: AccessContext): Subject {
    const acting = principalOf(context);
    if (acting === undefined) {
        throw new AccessError("FORBIDDEN", "the request needs an authenticated principal");
    }

    return acting;
}

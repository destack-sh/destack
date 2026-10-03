import type {} from "@destack/package/import-meta";
import { SYNC_PACKAGE, type Subject } from "@destack/sync";
import { grantersOf, none, relation, readersOf, union } from "./expression.ts";
import { Policy } from "./policy.ts";

/** The package declaring access's own types. */
const OWNER = import.meta.destack.package;

/** The identity of the package declaring access's own types. */
export const ACCESS_PACKAGE_ID = OWNER.id;

/** The kinds of principal: people, machines and installed software that authenticate, and the groups sharing reaches them through. */
export const principal = {
    /** A person, identified globally, and the scope with their own objects. */
    user: new Policy(OWNER, { name: "user", permissions: {}, scope: true, isGlobal: true }),
    /** A machine running Destack, living in its account, and the scope of its local operations. */
    host: new Policy(OWNER, { name: "host", permissions: {}, scope: true }),
    /** A region of Destack's hosted platform, administering the spaces placed in it. */
    region: new Policy(OWNER, { name: "region", permissions: {}, isGlobal: true }),
    /** An application installed into an account or space: the principal of software. */
    installation: new Policy(OWNER, { name: "installation", permissions: {} }),
    /** A space, the principal its cell copies rows for. */
    space: new Policy(OWNER, { name: "space", permissions: {}, isGlobal: true }),
    /** A host or region serving zones, as the directory knows it. */
    cell: new Policy(OWNER, { name: "cell", permissions: {}, isGlobal: true }),
    /** A non-person identity an account creates for automation, living in that account. */
    serviceAccount: new Policy(OWNER, { name: "service-account", permissions: {} }),
    /** A way to reach someone outside Destack that they prove control of, such as an email address. */
    contact: new Policy(OWNER, { name: "contact", permissions: {} }),
    /** A set of people, software and other groups that objects are shared with at once. */
    group: new Policy(OWNER, {
        name: "group",
        relations: { member: { subjects: ["user", "installation", "group#member"] } },
        permissions: {},
    }),
};

/** The universe, the root scope: roles and inherited rows bound on it apply in every scope. */
export const universe = new Policy(SYNC_PACKAGE, {
    name: "universe",
    permissions: {},
    scope: true,
});

/** Every caller related through its wildcard, signed in or anonymous, which links condition on a link secret. */
export const anyone = new Policy(OWNER, { name: "anyone", permissions: {} });

/** A named set of permissions, including the permissions of the roles it includes. */
export const role = new Policy(OWNER, {
    name: "role",
    relations: { includes: { subjects: ["role"] } },
    permissions: { create: none(), read: none(), update: none(), delete: none(), share: none() },
});

/** A relation or role binding between a subject and an object. */
export const relationship = new Policy(OWNER, {
    name: "relationship",
    relations: { subject: { subjects: [principal.user, principal.host, principal.installation] } },
    permissions: { read: union(relation("subject"), readersOf("object", "relation")) },
});

/** A proposed relationship awaiting acceptance. */
export const proposal = new Policy(OWNER, {
    name: "proposal",
    relations: {
        proposer: { subjects: [principal.user, principal.host, principal.installation] },
        addressee: {
            subjects: [principal.user, principal.host, principal.installation, principal.contact],
        },
        lender: { subjects: [principal.user, principal.host, principal.installation] },
    },
    permissions: {
        read: union(
            relation("proposer"),
            relation("addressee"),
            relation("lender"),
            grantersOf("object"),
        ),
    },
});

/** The policies of access's own types in every access model. */
export const INTRINSIC_POLICIES: readonly Policy[] = [
    universe,
    principal.user,
    principal.host,
    principal.region,
    principal.installation,
    principal.contact,
    principal.group,
    anyone,
    role,
    relationship,
    proposal,
];

/** Determine whether a subject is one principal rather than a set or another object. */
export function isPrincipal(subject: Subject): boolean {
    return Object.values(principal).some((kind) => kind.is(subject));
}

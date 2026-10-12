import type {} from "@destack/package/import-meta";
import { SYNC_PACKAGE, type Subject } from "@destack/sync";
import { grantersOf, none, relation, readersOf, union } from "./expression.ts";
import { Policy } from "./policy.ts";

/** The package declaring access's own types. */
const OWNER = import.meta.destack.package;

/** The identity of the package declaring access's own types. */
export const ACCESS_PACKAGE_ID = OWNER.id;

/** The kinds of principal: authenticating people, machines, clients and software, and groups. */
export const principal = {
    /** A person, identified globally, and the scope with their own objects. */
    user: new Policy(OWNER, { name: "user", permissions: {}, scope: true, isGlobal: true }),
    /** A machine running spaces, identified globally, and the scope of its local operations. */
    machine: new Policy(OWNER, { name: "machine", permissions: {}, scope: true, isGlobal: true }),
    /** A program a person signs in through, such as a browser profile, global by its key. */
    client: new Policy(OWNER, { name: "client", permissions: {}, scope: true, isGlobal: true }),
    /** An application installed into a space: the principal of software. */
    installation: new Policy(OWNER, { name: "installation", permissions: {} }),
    /** A space, which signs with its own key and keeps copies for itself. */
    space: new Policy(OWNER, { name: "space", permissions: {}, isGlobal: true }),
    /** A way to contact someone outside Destack with proven control, such as an email. */
    contact: new Policy(OWNER, { name: "contact", permissions: {} }),
    /** A set of people, software and other groups that objects are shared with at once. */
    group: new Policy(OWNER, {
        name: "group",
        relations: { member: { subjects: ["user", "installation", "group#member"] } },
        permissions: {},
    }),
};

/** The principal kinds that sign in, whose wildcards stand for every signed-in caller. */
const SIGNING = [
    principal.user,
    principal.machine,
    principal.client,
    principal.installation,
    principal.space,
];

/** Every signed-in caller: the wildcard of each signing principal kind. */
export const SIGNED_IN: readonly Subject[] = SIGNING.map((kind) => kind.reference("*", "*"));

/** The universe, the root scope, whose roles and inherited rows apply in every scope. */
export const universe = new Policy(SYNC_PACKAGE, {
    name: "universe",
    relations: {
        // every machine, and every signed-in caller, through their wildcards
        machine: { subjects: [principal.machine.all()] },
        "signed-in": { subjects: SIGNING.map((kind) => kind.all()) },
    },
    permissions: { grant: none() },
    grantedBy: "grant",
    scope: true,
});

/** Every caller through its wildcard, signed in or anonymous, as links on a link secret. */
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
    relations: {
        subject: { subjects: [principal.user, principal.machine, principal.installation] },
    },
    permissions: { read: union(relation("subject"), readersOf("object", "relation")) },
});

/** A pending grant of a relationship, awaiting its addressee or a grantor. */
export const invitation = new Policy(OWNER, {
    name: "invitation",
    relations: {
        inviter: { subjects: [principal.user, principal.machine, principal.installation] },
        addressee: {
            subjects: [
                principal.user,
                principal.machine,
                principal.installation,
                principal.contact,
            ],
        },
        lender: { subjects: [principal.user, principal.machine, principal.installation] },
    },
    permissions: {
        read: union(
            relation("inviter"),
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
    principal.machine,
    principal.client,
    principal.installation,
    principal.space,
    principal.contact,
    principal.group,
    anyone,
    role,
    relationship,
    invitation,
];

/** Determine whether a subject is one principal rather than a set or another object. */
export function isPrincipal(subject: Subject): boolean {
    return Object.values(principal).some((kind) => kind.is(subject));
}

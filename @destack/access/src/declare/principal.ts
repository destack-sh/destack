import type {} from "@destack/package/import-meta";
import type { DeclarationName, PackageId } from "@destack/package";
import { type DigestCrypto, Identifier, type Uuid } from "@destack/schema";
import { type ObjectReference, Scope, SYNC_PACKAGE, type Subject } from "@destack/sync";
import { grantersOf, none, relation, readersOf, union } from "./expression.ts";
import { Policy } from "./policy.ts";

/** Destack's installation namespace: the DNS namespace's UUIDv8 of installation.destack.app. */
const INSTALLATION_NAMESPACE: Uuid = "f04d5c7d-9569-8bab-8a86-9074317ddc54";

/** The package declaring access's types. */
const OWNER = import.meta.destack.package;

/** The identity of the package declaring access's types. */
export const ACCESS_PACKAGE_ID = OWNER.id;

/** The kinds of principal: people, machines, clients, devices, software and groups. */
export const principal = {
    /** A person, identified globally, and the scope with their objects. */
    user: new Policy(OWNER, { name: "user", permissions: {}, scope: true, isGlobal: true }),
    /** A machine running spaces, identified globally, and the scope of its local operations. */
    machine: new Policy(OWNER, {
        name: "machine",
        permissions: {},
        scope: true,
        isGlobal: true,
        partition: true,
    }),
    /** A person's registration of a program acting for them, such as a browser or a token. */
    client: new Policy(OWNER, { name: "client", permissions: {}, isGlobal: true }),
    /** A program instance on a computer, the scope of its local objects for everyone using it. */
    device: new Policy(OWNER, {
        name: "device",
        permissions: {},
        scope: true,
        isGlobal: true,
        partition: true,
    }),
    /** An application installed into a space: the principal of software. */
    installation: new Policy(OWNER, { name: "installation", permissions: {} }),
    /** A space, which signs with its key and stores replicas for itself. */
    space: new Policy(OWNER, {
        name: "space",
        permissions: {},
        scope: true,
        isGlobal: true,
        partition: true,
    }),
    /** A way to contact someone outside Destack with proven control, such as an email. */
    contact: new Policy(OWNER, { name: "contact", permissions: {} }),
    /** The principal of every caller who proves no identity, like Kubernetes' anonymous user. */
    anonymous: new Policy(OWNER, { name: "anonymous", permissions: {}, isGlobal: true }),
    /** A set of people, software and other groups that objects are shared with at once. */
    group: new Policy(OWNER, {
        name: "group",
        relations: { member: { subjects: ["user", "installation", "group#member"] } },
        permissions: {},
    }),
};

/** Packages as the sets of their installations in a scope, the code serving their types. */
export const installedPackage = new Policy(OWNER, {
    name: "package",
    relations: { installation: { subjects: [principal.installation] } },
    permissions: {},
});

/** The subject of every caller who proves no identity. */
export const ANONYMOUS: Subject = principal.anonymous.reference(Scope.universe.id, "anonymous");

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

/** The subject types of every signed-in caller, the wildcards a relation to them accepts. */
export const SIGNED_IN_TYPES = SIGNING.map((kind) => kind.all());

/** The universe, the root scope, whose roles and inherited rows apply in every scope. */
export const universe = new Policy(SYNC_PACKAGE, {
    name: "universe",
    relations: {
        // every machine, and every signed-in caller, through their wildcards
        machine: { subjects: [principal.machine.all()] },
        "signed-in": { subjects: SIGNED_IN_TYPES },
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
    permissions: {
        create: none(),
        read: none(),
        update: none(),
        delete: none(),
        share: none(),
    },
});

/** A relation or role binding between a subject and an object. */
export const relationship = new Policy(OWNER, {
    name: "relationship",
    relations: {
        subject: { subjects: [principal.user, principal.machine, principal.installation] },
    },
    permissions: {
        read: union(relation("subject"), readersOf("object", "relation")),
        grant: grantersOf("object"),
    },
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
    },
    permissions: {
        read: union(relation("inviter"), relation("addressee"), grantersOf("object")),
        grant: grantersOf("object"),
    },
});

/** A delegator's grant of permissions to a principal acting for it, revoked by the delegator. */
export const grant = new Policy(OWNER, {
    name: "grant",
    relations: {
        delegator: { subjects: [principal.user, principal.machine, principal.installation] },
        delegate: {
            subjects: [principal.user, principal.machine, principal.client, principal.installation],
            onDelete: "cascade",
        },
        "delegate-space": { subjects: [principal.space], onDelete: "cascade" },
    },
    permissions: {
        create: none(),
        read: union(relation("delegator"), relation("delegate"), relation("delegate-space")),
        revoke: relation("delegator"),
    },
});

/** The policies of access's types in every access model. */
export const INTRINSIC_POLICIES: readonly Policy[] = [
    universe,
    principal.user,
    principal.machine,
    principal.client,
    principal.device,
    principal.installation,
    principal.space,
    principal.contact,
    principal.anonymous,
    principal.group,
    anyone,
    role,
    relationship,
    invitation,
    grant,
];

/** The identifiers of installations, from the scope that has them. */
export const InstallationId = {
    /** Identify the base installation of a package in a scope, born with the scope. */
    base(
        scope: Pick<ObjectReference, "id">,
        packageId: PackageId,
        crypto: DigestCrypto,
    ): Promise<Identifier<"installation">> {
        return Identifier.hash(
            "installation",
            INSTALLATION_NAMESPACE,
            `${scope.id}/base/${packageId}`,
            crypto,
        );
    },

    /** Identify the installation a scope's stack declares under a name. */
    declared(
        scope: Pick<ObjectReference, "id">,
        name: DeclarationName,
        crypto: DigestCrypto,
    ): Promise<Identifier<"installation">> {
        return Identifier.hash(
            "installation",
            INSTALLATION_NAMESPACE,
            `${scope.id}/declared/${name}`,
            crypto,
        );
    },
};

/** Determine whether a subject is one principal rather than a set or another object. */
export function isPrincipal(subject: Subject): boolean {
    return Object.values(principal).some((kind) => kind.is(subject));
}

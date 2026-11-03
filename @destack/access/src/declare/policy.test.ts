import { expect, test } from "@destack/test";
import { Authorizer, INTRINSIC_POLICIES, none, Policy, principal, relation } from "../index.ts";
import { account, group, module1, node, team } from "../test/fixture.ts";

/** List the names of some policies. */
function names(policies: readonly { readonly name: string }[]): string[] {
    return policies.map((policy) => policy.name);
}

test("include the policies declared policies reference, transitively", () => {
    // declare notes viewed by groups of accounts, whose residents are regions' servers
    const authorizer = new Authorizer([node, team]);

    // include every intrinsic, declared and referenced policy once
    expect(names(authorizer.policies()).toSorted()).toEqual(
        [...names(INTRINSIC_POLICIES), "node", "team", "group", "account", "region"].toSorted(),
    );
    expect(authorizer.policy(account.reference("universe", "account-1"))).toBe(account);
    expect(authorizer.policy(group.reference("space-1", "group-1"))).toBe(group);
});

test("take the policy a package declares representing a principal over access's own", () => {
    // list the intrinsic user before the policy representing it
    const user = new Policy(principal.user.package, {
        name: principal.user.name,
        permissions: { profile: none() },
    });
    const crew = new Policy(module1.package, {
        name: "crew",
        relations: { member: { subjects: [user] } },
        permissions: { read: relation("member") },
    });
    const document = new Policy(module1.package, {
        name: "document",
        relations: { viewer: { subjects: [principal.user, crew] } },
        permissions: { read: relation("viewer") },
    });
    expect(new Authorizer([document]).policy(principal.user.reference("universe", "u"))).toBe(user);
});

/** Declare a document its viewers read, as each build loading it does. */
function viewedDocument(): Policy {
    return new Policy(module1.package, {
        name: "document",
        relations: { viewer: { subjects: [principal.user] } },
        permissions: { read: relation("viewer") },
    });
}

test("take copies of a policy as one type, as each build loads its own", () => {
    // declare one document type twice, as two builds load it
    const [document, copy] = [viewedDocument(), viewedDocument()];

    // keep one policy for both copies, by the package and name they declare
    const authorizer = new Authorizer([document, copy]);
    expect(authorizer.policy({ packageId: module1.package.id, type: "document" })).toBe(document);
});

test("grant relations through the policy's grant permission unless they declare another or none", () => {
    // declare a shared folder with system-related owners and root ownership
    const folder = new Policy(module1.package, {
        name: "folder",
        relations: {
            member: { subjects: [principal.user] },
            root: { subjects: [principal.user], grantedBy: "administer" },
            owner: { subjects: [principal.user], grantedBy: null },
        },
        permissions: {
            share: relation("member"),
            administer: relation("root"),
            read: relation("owner"),
        },
        grantedBy: "share",
    });

    // resolve each relation's grant permission, none for the system-related one
    expect(
        Object.entries(folder.definition.relations).map(([name, entry]) => [name, entry.grantedBy]),
    ).toEqual([
        ["member", "share"],
        ["root", "administer"],
        ["owner", undefined],
    ]);
});

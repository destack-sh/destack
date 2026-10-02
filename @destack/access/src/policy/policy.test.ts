import { expect, test } from "@destack/test";
import { Authorizer, INTRINSIC_POLICIES, none, Policy, principal, relation } from "../index.ts";
import { account, group, module1, node, team } from "../test/fixture.ts";

/** List the names of some policies. */
function names(policies: readonly { readonly name: string }[]): string[] {
    return policies.map((policy) => policy.name);
}

test("include the policies declared policies reference, transitively", () => {
    // declare notes and teams: note viewers reference groups of accounts
    const authorizer = new Authorizer([node, team]);

    // include every intrinsic, declared and referenced policy once
    expect(names(authorizer.policies()).toSorted()).toEqual(
        [...names(INTRINSIC_POLICIES), "node", "team", "group", "account"].toSorted(),
    );
    expect(authorizer.policy(account.reference("universe", "account-1"))).toBe(account);
    expect(authorizer.policy(group.reference("space-1", "group-1"))).toBe(group);
});

test("prefer a policy representing a principal and refuse two policies for one type", () => {
    // reach the intrinsic user before the policy representing it
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

    // refuse a second policy declaring the same type
    const copy = new Policy(module1.package, { name: "document", permissions: {} });
    expect(() => new Authorizer([document, copy])).toThrow(
        `duplicate object type: ${JSON.stringify([module1.package.id, "document"])}`,
    );
});

test("grant relations through the policy's grant permission unless they name another or none", () => {
    // declare a shared folder with system-related owners and root ownership
    const folder = new Policy(module1.package, {
        name: "folder",
        relations: {
            member: { subjects: [principal.user] },
            root: { subjects: [principal.user], grantedBy: "own" },
            owner: { subjects: [principal.user], grantedBy: null },
        },
        permissions: { share: relation("member"), own: relation("root"), read: relation("owner") },
        grantedBy: "share",
    });

    // resolve each relation's grant permission, none for the system-related one
    expect(
        Object.entries(folder.definition.relations).map(([name, entry]) => [name, entry.grantedBy]),
    ).toEqual([
        ["member", "share"],
        ["root", "own"],
        ["owner", undefined],
    ]);
});

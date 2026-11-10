import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { GrantDeclaration, PermissionSet } from "./grant.ts";

/** The package declaring the requested types. */
const NOTES = PackageId.parse("package-01996ab0-0000-7000-8000-000000000001");

test("grant a permission set in its scopes, the home's in each home scope and the universe's in every scope, dropping those of an unknown home or account", () => {
    // request a permission in the space, the person's home, the space's account and the universe
    const packageId = NOTES;
    const request = PermissionSet.parse({
        space: [{ packageId, type: "note", name: "read" }],
        home: [{ packageId, type: "notification", name: "read" }],
        account: [{ packageId, type: "member", name: "read" }],
        universe: [{ packageId, type: "user", name: "recipient" }],
    });

    // grant each in its scope with its scope in the set, and none of an unknown home or account
    const note = { packageId, type: "note", name: "read" };
    const member = { packageId, type: "member", name: "read" };
    const recipient = { packageId, type: "user", name: "recipient" };
    const notification = { packageId, type: "notification", name: "read" };
    expect([
        PermissionSet.grant(request, {
            space: "space-a",
            home: ["user-a", "space-home"],
            account: "account-a",
        }),
        PermissionSet.grant(request, { space: "space-a", home: [], account: undefined }),
    ]).toEqual([
        [
            { within: "space", permission: { ...note, scope: "space-a" } },
            { within: "account", permission: { ...member, scope: "account-a" } },
            { within: "universe", permission: recipient },
            { within: "home", permission: { ...notification, scope: "user-a" } },
            { within: "home", permission: { ...notification, scope: "space-home" } },
        ],
        [
            { within: "space", permission: { ...note, scope: "space-a" } },
            { within: "universe", permission: recipient },
        ],
    ]);
});

test("accept a package's request of each delegator's scopes, refusing a scope that delegator cannot lend", () => {
    // request the scopes a space grants at install and a user's on use, their profile in the universe among them
    const packageId = NOTES;
    const read = (type: string) => [{ packageId, type, name: "read" }];
    const accepted = GrantDeclaration.safeParse({
        space: { space: read("note"), universe: read("user") },
        user: { home: read("notification"), universe: read("user") },
    });

    // refuse a home the space cannot lend
    const home = GrantDeclaration.safeParse({ space: { home: read("notification") } });
    expect([
        accepted.success,
        home.error?.issues.map((issue) => [issue.code, issue.path.join(".")]),
    ]).toEqual([true, [["unrecognized_keys", "space"]]]);
});

import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { PermissionRequest, PermissionSet } from "./request.ts";

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

    // grant each in its scope, and only the space's and universe's without a home or an account
    expect([
        PermissionSet.grant(request, {
            space: "space-a",
            home: ["user-a", "space-home"],
            account: "account-a",
        }),
        PermissionSet.grant(request, { space: "space-a", home: [], account: undefined }),
    ]).toEqual([
        [
            { packageId, type: "note", name: "read", scope: "space-a" },
            { packageId, type: "member", name: "read", scope: "account-a" },
            { packageId, type: "user", name: "recipient" },
            { packageId, type: "notification", name: "read", scope: "user-a" },
            { packageId, type: "notification", name: "read", scope: "space-home" },
        ],
        [
            { packageId, type: "note", name: "read", scope: "space-a" },
            { packageId, type: "user", name: "recipient" },
        ],
    ]);
});

test("accept a package's request of each delegator's scopes, refusing a scope that delegator cannot lend", () => {
    // request the space's lendable scopes for the installation and a user's for the user
    const packageId = NOTES;
    const read = (type: string) => [{ packageId, type, name: "read" }];
    const accepted = PermissionRequest.safeParse({
        installation: { space: read("note"), universe: read("user") },
        user: { home: read("notification") },
    });

    // refuse a home the space cannot lend, and the universe a user does not lend
    const home = PermissionRequest.safeParse({ installation: { home: read("notification") } });
    const universe = PermissionRequest.safeParse({ user: { universe: read("user") } });
    expect([
        accepted.success,
        home.error?.issues.map((issue) => [issue.code, issue.path.join(".")]),
        universe.error?.issues.map((issue) => [issue.code, issue.path.join(".")]),
    ]).toEqual([true, [["unrecognized_keys", "installation"]], [["unrecognized_keys", "user"]]]);
});

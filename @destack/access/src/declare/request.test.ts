import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { PermissionRequest } from "./request.ts";

/** The package declaring the requested types. */
const NOTES = PackageId.parse("package-01996ab0-0000-7000-8000-000000000001");

test("grant requested permissions in their scopes, the home's in each home scope and the universe's in every scope, dropping those of an unknown home or account", () => {
    // request a permission in the space, the person's home, the space's account and the universe
    const packageId = NOTES;
    const request = PermissionRequest.parse({
        space: [{ packageId, type: "note", name: "read" }],
        home: [{ packageId, type: "notification", name: "read" }],
        account: [{ packageId, type: "member", name: "read" }],
        universe: [{ packageId, type: "user", name: "recipient" }],
    });

    // grant each in its scope, and only the space's and universe's without a home or an account
    expect([
        PermissionRequest.grant(request, {
            space: "space-a",
            home: ["user-a", "space-home"],
            account: "account-a",
        }),
        PermissionRequest.grant(request, { space: "space-a", home: [], account: undefined }),
    ]).toEqual([
        [
            { packageId, type: "note", name: "read", scope: "space-a" },
            { packageId, type: "notification", name: "read", scope: "user-a" },
            { packageId, type: "notification", name: "read", scope: "space-home" },
            { packageId, type: "member", name: "read", scope: "account-a" },
            { packageId, type: "user", name: "recipient" },
        ],
        [
            { packageId, type: "note", name: "read", scope: "space-a" },
            { packageId, type: "user", name: "recipient" },
        ],
    ]);
});

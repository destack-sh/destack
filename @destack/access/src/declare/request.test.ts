import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { PermissionRequest } from "./request.ts";

/** The package declaring the requested types. */
const NOTES = PackageId.parse("package-01996ab0-0000-7000-8000-000000000001");

test("grant requested permissions in their scopes, dropping those of an unknown home or account", () => {
    // request a permission in the space, one in the person's home and one in the space's account
    const packageId = NOTES;
    const request = {
        space: [{ packageId, type: "note", name: "read" }],
        home: [{ packageId, type: "notification", name: "read" }],
        account: [{ packageId, type: "member", name: "read" }],
    };

    // grant each in its scope, and only the space's without a home or an account
    expect([
        PermissionRequest.grant(request, {
            space: "space-a",
            home: "space-home",
            account: "account-a",
        }),
        PermissionRequest.grant(request, { space: "space-a", home: undefined, account: undefined }),
    ]).toEqual([
        [
            { packageId, type: "note", name: "read", scope: "space-a" },
            { packageId, type: "notification", name: "read", scope: "space-home" },
            { packageId, type: "member", name: "read", scope: "account-a" },
        ],
        [{ packageId, type: "note", name: "read", scope: "space-a" }],
    ]);
});

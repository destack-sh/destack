import { expect, test } from "@destack/test";
import { PackageId } from "../definition/package.ts";
import { ViewDescription } from "./view.ts";

/** The package declaring the presented note type. */
const NOTES = PackageId.parse("package-01996ab0-0000-7000-8000-000000000001");

test("rank views presenting no type first, then by their strongest presentation, then by name", () => {
    const note = (priority: "default" | "option") => ({
        packageId: NOTES,
        type: "note",
        priority,
    });
    expect(
        ViewDescription.rank({
            preview: { presents: [note("option")] },
            editor: { presents: [note("option"), note("default")] },
            notes: { presents: [] },
            archive: { presents: [] },
            reader: { presents: [note("default")] },
        }),
    ).toEqual(["archive", "notes", "editor", "reader", "preview"]);
    expect(ViewDescription.rank({})).toEqual([]);
});

test("grant a view's permissions in their scopes, dropping those of an unknown home or account", () => {
    // request a permission in the space, one in the person's home and one in the space's account
    const packageId = NOTES;
    const view = {
        permissions: {
            space: [{ packageId, type: "note", name: "read" }],
            home: [{ packageId, type: "notification", name: "read" }],
            account: [{ packageId, type: "member", name: "read" }],
        },
    };

    // grant each in its scope, and only the space's without a home or an account
    expect([
        ViewDescription.grants(view, {
            space: "space-a",
            home: "space-home",
            account: "account-a",
        }),
        ViewDescription.grants(view, { space: "space-a", home: undefined, account: undefined }),
    ]).toEqual([
        [
            { packageId, type: "note", name: "read", scope: "space-a" },
            { packageId, type: "notification", name: "read", scope: "space-home" },
            { packageId, type: "member", name: "read", scope: "account-a" },
        ],
        [{ packageId, type: "note", name: "read", scope: "space-a" }],
    ]);
});

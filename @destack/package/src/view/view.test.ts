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

test("grant a view's home permissions in the person's home, none without one, and the rest in its space", () => {
    // request a permission on a space type and one on a home type
    const packageId = NOTES;
    const view = {
        permissions: [
            { packageId, type: "note", name: "read" },
            { packageId, type: "notification", name: "read" },
        ],
        home: [{ packageId, type: "notification" }],
    };

    // scope each permission, dropping the home one without a home
    expect([
        ViewDescription.grants(view, { space: "space-a", home: "space-home" }),
        ViewDescription.grants(view, { space: "space-a", home: undefined }),
    ]).toEqual([
        [
            { packageId, type: "note", name: "read", scope: "space-a" },
            { packageId, type: "notification", name: "read", scope: "space-home" },
        ],
        [{ packageId, type: "note", name: "read", scope: "space-a" }],
    ]);
});

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

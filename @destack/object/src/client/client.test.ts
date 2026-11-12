import { argument, plural, t } from "@destack/locale";
import { PackageVersion } from "@destack/package";
import { expect, test } from "@destack/test";
import { field } from "../field/index.ts";
import { defineObject } from "../object/index.ts";
import { ObjectClient } from "./client.ts";

/** Declare a note type of a package, as two packages may both name one. */
function noteOf(id: string, name: string) {
    const owner = PackageVersion.parse({ id, name, version: "2026.11.0" });

    return defineObject(
        {
            name: "note",
            plural: "notes",
            title: t`${plural(argument("count"), { one: "Note", other: "Notes" })}`,
            container: "scope",
            fields: { title: field.string() },
        },
        undefined,
        { package: owner },
    );
}

test("key two packages' same-named object types apart, keeping both", () => {
    const notes = noteOf("package-01996ab0-0000-7000-8000-0000000000a1", "@example/notes");
    const journal = noteOf("package-01996ab0-0000-7000-8000-0000000000a2", "@example/journal");

    // keep both notes, each under its package-qualified key
    const keyed = ObjectClient.keyed([notes, journal]);
    expect(Object.values(keyed)).toEqual([notes, journal]);
});

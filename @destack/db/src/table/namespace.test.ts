import { expect, test } from "@destack/test";
import { boundedName } from "./namespace.ts";

test("fit derived names within the identifier limit and keep them distinct", () => {
    const prefix = "destack__access__role_permission_role_id_destack__access__role";
    const first = boundedName(`${prefix}_id_fk`);
    const second = boundedName(`${prefix}_key_fk`);

    // keep short names and shorten long ones to the limit
    expect(boundedName("note_pk")).toBe("note_pk");
    expect([first.length, second.length]).toEqual([63, 63]);
    expect(boundedName(`${prefix}_id_fk`)).toBe(first);

    // distinguish names sharing the kept prefix
    expect(first.slice(0, 54)).toBe(second.slice(0, 54));
    expect(first).not.toBe(second);
});

test("leave room for suffixes appended to a bounded name", () => {
    const name = "destack__account__account__destack__access__relationship_object_id";

    // shorten long names to the limit minus the reserved suffix
    expect([
        boundedName(name, "_maintain".length).length,
        `${boundedName(name, "_maintain".length)}_maintain`.length,
        boundedName("note_count", "_maintain".length),
    ]).toEqual([54, 63, "note_count"]);
});

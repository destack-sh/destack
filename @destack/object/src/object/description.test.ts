import { principal } from "@destack/access";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { field } from "../field/index.ts";
import { defineObject } from "./index.ts";
import { ObjectDescription } from "./description.ts";

/** Define a type of notes in some scopes. */
function notes(scope: NonNullable<Parameters<typeof defineObject>[0]["scope"]>) {
    return defineObject({
        name: "note",
        plural: "notes",
        scope,
        fields: { title: field.string(schema.string()) },
        permissions: {},
    });
}

test("report a type stored in a space when a space is among its scopes", () => {
    expect(
        [
            notes([principal.space]),
            notes([principal.user, principal.space]),
            notes([principal.user]),
            notes([principal.user, principal.device]),
        ].map((type) => ObjectDescription.isInSpace(ObjectDescription.of(type))),
    ).toEqual([true, true, false, false]);
});

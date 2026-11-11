import { principal, relation } from "@destack/access";
import { t } from "@destack/locale";
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

/** Define notes describing a permission they lack, and validate them. */
function misdescribedNotes() {
    defineObject({
        name: "note",
        plural: "notes",
        permissions: { read: relation("owner") },
        descriptions: { delete: t`Delete your notes` },
    }).requireValid();
}

test("describe a type's permissions from their verbs and its plural, or as it declares, refusing a description of none", () => {
    // define note cards whose share permission the type describes itself
    const cards = defineObject({
        name: "card",
        plural: "noteCards",
        fields: { title: field.string(schema.string()) },
        permissions: {
            read: relation("owner"),
            manage: relation("owner"),
            archive: relation("owner"),
            share: relation("owner"),
        },
        descriptions: { share: t`Share your notes with others` },
    });
    const described = ObjectDescription.of(cards);

    // read the standard verbs, a custom permission's own name, and the declared message's source
    expect({
        sources: ["read", "manage", "archive", "share"].map(
            (name) => ObjectDescription.describe(described, name).source,
        ),
        id: ObjectDescription.describe(described, "read").id,
    }).toEqual({
        sources: [
            "See note cards",
            "Change note cards",
            "Archive note cards",
            "Share your notes with others",
        ],
        id: "permission.card.read",
    });
    expect(misdescribedNotes).toThrow(
        new TypeError("object 'note' describes no permission 'delete'"),
    );
});

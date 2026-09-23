import { defineSpace } from "@destack/space";
import type {} from "@destack/package/import-meta";
import { defineAccount } from "@destack/model/declare";
import { defineObject, relation } from "@destack/access";
import { database } from "@destack/template-stack";
import {
    defineSetting,
    defineSettingAssignment,
    defineSettingPolicy,
} from "@destack/setting/declare";
import { schema, identifier } from "@destack/schema";

/** Describe a shared setting without reading runtime values. */
export const language = defineSetting({
    package: import.meta.destack.package,
    name: "language",
    title: "Language",
    description: "Language used for shared documents.",
    schema: schema.enum(["en", "de"]),
    default: "en",
    scope: "space",
    overrides: ["installation"],
    apply: "immediate",
});

/** Configure the same declared value through typed source authoring. */
export const languageAssignment = defineSettingAssignment(
    language,
    {
        kind: "space",
        location: {
            spaceId: identifier("space").parse("space-019f5530-8000-7000-8000-000000000003"),
        },
    },
    "de",
);

/** Describe a recommendation independently of the persisted assignment. */
export const languagePolicy = defineSettingPolicy(
    language,
    {
        authority: {
            kind: "space",
            spaceId: identifier("space").parse("space-019f5530-8000-7000-8000-000000000003"),
        },
        mode: "recommended",
    },
    "en",
);

/** Reuse the database declaration from the shared stack package. */
export { database } from "@destack/template-stack";

/** Declare environments independently of the destination space. */
export const account = defineAccount({ environments: { development: {}, production: {} } });

/** Declare inspectable application permissions. */
export const note = defineObject({
    packageId: import.meta.destack.package.id,
    name: "note",
    attributes: {},
    relations: { owner: { kind: "subject", subjects: ["user"] } },
    permissions: { read: relation("owner") },
});

/** Configure a space without provisioning resources during compilation. */
export const personal = defineSpace({
    resources: {
        main: { declaration: database, retention: "retain", tags: {} },
    },
});

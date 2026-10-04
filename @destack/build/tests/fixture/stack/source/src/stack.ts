import { defineSpace, install } from "@destack/space";
import stack from "./package.ts";
import type {} from "@destack/package/import-meta";
import { defineAccount } from "@destack/account/declare";
import { database } from "@template/stack";
import { defineSetting } from "@destack/setting/declare";
import { schema } from "@destack/schema";

/** Describe a shared setting without reading runtime values. */
export const language = defineSetting({
    name: "language",
    title: "Language",
    description: "Language used for shared documents.",
    schema: schema.enum(["en", "de"]),
    default: "en",
    scope: "space",
    overrides: ["installation"],
    apply: "immediate",
});

/** Reuse the database declaration from the shared stack package. */
export { database } from "@template/stack";

/** Declare environments independently of the destination space. */
export const account = defineAccount({ environments: { development: {}, production: {} } });

/** Configure a space without provisioning resources during compilation. */
export const personal = defineSpace({
    resources: {
        main: { declaration: database, retention: { within: { days: 30 } }, tags: {} },
    },
    installations: {
        notes: install(stack, { main: "main" }),
    },
    settings: {
        language: { setting: language, value: "de", mode: "set" },
        "language-policy": { setting: language, value: "en", mode: "recommend" },
    },
});

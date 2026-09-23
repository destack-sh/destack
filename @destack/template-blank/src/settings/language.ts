import { defineSetting } from "@destack/setting/declare";
import { schema } from "@destack/schema";
import type {} from "@destack/package/import-meta";

/** Language selected for generated content. */
export const language = defineSetting({
    package: import.meta.destack.package,
    name: "language",
    title: "Content language",
    description: "Choose the language used when creating content.",
    schema: schema.enum(["en", "de", "fr"]),
    default: "en",
    scope: "user",
    overrides: ["space", "installation"],
    apply: "immediate",
});

import { defineSetting } from "@destack/setting/declare";
import { schema } from "@destack/schema";
import type {} from "@destack/package/import-meta";
import { t } from "@destack/locale";

/** Language selected for generated content. */
export const language = defineSetting({
    name: "language",
    title: t`Content language`,
    description: t`Choose the language used when creating content.`,
    schema: schema.enum(["en", "de", "fr"]),
    default: "en",
    scope: "user",
    overrides: ["space", "installation"],
});

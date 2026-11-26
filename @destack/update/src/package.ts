import { t } from "@destack/locale";
import { definePackage } from "@destack/object";
import { checkForUpdates, distribution, restartToUpdate } from "./object/index.ts";
import { UpdateError } from "./error/error.ts";

/** The handle machines serve their distribution through, and clients call it by. */
export default definePackage({
    title: t.context("title")`Update`,
    description: t`Update installed Destack distributions from a signed release feed.`,
    icon: "arrows-clockwise",
    color: "brown",
    objects: [distribution],
    actions: [checkForUpdates, restartToUpdate],
    errors: UpdateError,
});

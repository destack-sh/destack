import { t } from "@destack/locale";
import { definePackage } from "@destack/object";
import { checkForUpdates, distribution, restartToUpdate } from "./object/index.ts";
import { UpdateError } from "./error/error.ts";

/** The handle machines serve their distribution through, and clients call it by. */
export default definePackage({
    title: t`Update`,
    description: t`Update installed Destack distributions from a signed release feed.`,
    icon: "arrows-clockwise",
    color: "bronze",
    objects: [distribution],
    commands: [checkForUpdates, restartToUpdate],
    errors: UpdateError,
});

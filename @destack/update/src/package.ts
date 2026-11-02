import { definePackage } from "@destack/object";
import { checkForUpdates, distribution, restartToUpdate } from "./object/index.ts";
import { UpdateError } from "./error/error.ts";

/** The handle machines serve their distribution through, and clients call it by. */
export default definePackage({
    objects: { distribution },
    commands: { "check-for-updates": checkForUpdates, "restart-to-update": restartToUpdate },
    errors: UpdateError,
});

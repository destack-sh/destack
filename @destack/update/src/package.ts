import { definePackage } from "@destack/object";
import { distribution } from "./object/index.ts";
import { UpdateError } from "./error/error.ts";

/** The handle machines serve their distribution through, and clients call it by. */
export default definePackage({
    objects: { distribution },
    errors: UpdateError,
});

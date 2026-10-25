import { definePackage } from "@destack/object";
import { SocialError } from "./error/error.ts";
import { comment, favourite, presence, reaction, receipt } from "./object/index.ts";

/** The social package: comments, reactions, read receipts, favourites and presence. */
export default definePackage({
    objects: { comment, reaction, receipt, favourite, presence },
    errors: SocialError,
});

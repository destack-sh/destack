import { t } from "@destack/locale";
import { definePackage } from "@destack/object";
import { SocialError } from "./error/error.ts";
import { comment, favourite, presence, reaction, receipt } from "./object/index.ts";

/** The social package: comments, reactions, read receipts, favourites and presence. */
export default definePackage({
    title: t`Social`,
    description: t`Attach comments, reactions, receipts, favourites and presence to any object, notifying mentioned and participating principals.`,
    icon: "chats-teardrop",
    color: "violet",
    objects: [comment, reaction, receipt, favourite, presence],
    errors: SocialError,
});

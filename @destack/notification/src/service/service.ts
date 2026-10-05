import { defineService } from "@destack/service";
import { delivery } from "../object/delivery.ts";
import { notification } from "../object/activity.ts";

/** A person's inbox in their home: the notifications other spaces' activities project into it, and their deliveries. */
export const inboxService = defineService("inbox", {
    objects: { notification, delivery },
});

import { ModuleMetadata } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { NotificationType, type NotificationDefinition } from "../notification/notification.ts";
import { NotificationName } from "../object/activity.ts";
import { INTERRUPTION_LEVELS, Preference } from "../preference/preference.ts";

/** A notification's metadata. */
export const NotificationMetadata = defineSchema(
    schema.object({
        /** The name, in camel case. */
        name: NotificationName,
        /** The label settings show. */
        title: schema.string().min(1),
        /** What it tells. */
        description: schema.string().min(1),
        /** How strongly it interrupts. */
        interruption: schema.enum(INTERRUPTION_LEVELS),
        /** The preference recipients start from. */
        preference: Preference,
    }),
);

/** Declare a notification. */
export function defineNotification<Payload extends schema.Schema>(
    definition: NotificationDefinition<Payload>,
    module?: ModuleMetadata,
): NotificationType<Payload> {
    // stamp the declaring package
    const owner = ModuleMetadata.require(module, "defineNotification").package;

    // validate the metadata and the action names
    const { name, title, description, interruption, preference, payload, actions } = definition;
    NotificationMetadata.parse({ name, title, description, interruption, preference });
    for (const action of Object.keys(actions ?? {})) {
        NotificationName.parse(action);
    }

    // require a declarative payload schema
    defineSchema(payload);

    // validate the action metadata as the declaration derives it
    return new NotificationType(owner, definition);
}

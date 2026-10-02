import { ModuleMetadata } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { Notification, type NotificationDefinition } from "../notification/notification.ts";
import { NotificationName } from "../object/notification.ts";
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

/** An action's metadata. */
export const ActionMetadata = defineSchema(
    schema.object({
        /** The button's label. */
        title: schema.string().min(1),
        /** Whether the action destroys or declines. */
        isDestructive: schema.boolean().optional(),
        /** The text field it asks for. */
        text: schema
            .object({
                /** The field's placeholder. */
                placeholder: schema.string().min(1),
                /** The send button's label. */
                button: schema.string().min(1),
            })
            .optional(),
    }),
);

/** Declare a notification. */
export function defineNotification<Payload extends schema.Schema>(
    definition: NotificationDefinition<Payload>,
    module?: ModuleMetadata,
): Notification<Payload> {
    // stamp the declaring package
    const owner = ModuleMetadata.require(module, "defineNotification").package;

    // validate the metadata
    const { name, title, description, interruption, preference, payload, actions } = definition;
    NotificationMetadata.parse({ name, title, description, interruption, preference });
    for (const [action, declared] of Object.entries(actions ?? {})) {
        NotificationName.parse(action);
        ActionMetadata.parse({
            title: declared.title,
            isDestructive: declared.isDestructive,
            text: declared.text,
        });
    }

    // require a declarative payload schema
    defineSchema(payload);

    return new Notification(owner, definition);
}

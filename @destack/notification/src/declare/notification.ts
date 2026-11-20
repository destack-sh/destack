import { ModuleMetadata } from "@destack/package";
import { schema } from "@destack/schema";
import {
    NotificationActionDescription,
    type NotificationActionDefinition,
} from "../notification/content.ts";
import {
    NotificationMetadata,
    NotificationType,
    type NotificationDefinition,
} from "../notification/notification.ts";

/** Declare a notification. */
export function defineNotification<Payload extends schema.Schema>(
    definition: NotificationDefinition<Payload>,
    module?: ModuleMetadata,
): NotificationType<Payload> {
    // stamp the declaring package
    const owner = ModuleMetadata.require(module, "defineNotification").package;

    // validate the metadata
    const { actions = [], ...declared } = definition;
    const { name, title, description, interruption, preference } = declared;
    NotificationMetadata.parse({ name, title, description, interruption, preference });

    // describe each action's button and call, requiring a mutating method of the declaring package
    const described = actions.map((given: NotificationActionDefinition<schema.Infer<Payload>>) => {
        // read the action and its call
        const { action, target, text } = given;
        const { call } = action;
        // refuse an action the open view handles, which calls nothing
        if (call === undefined) {
            throw new TypeError(`action '${action.name}' of '${name}' calls no method`);
        }
        // refuse another package's type
        else if (call.object.package.id !== owner.id) {
            throw new TypeError(
                `action '${action.name}' of '${name}' calls ${call.object.name} of another package`,
            );
        }
        // refuse a method changing nothing
        else if (!call.object.method(call.method).mutates) {
            throw new TypeError(
                `action '${action.name}' of '${name}' calls '${call.object.name}.${call.method}', which changes nothing`,
            );
        }

        return NotificationActionDescription.parse({
            name: action.name,
            title: action.title,
            ...(action.isDestructive ? { isDestructive: true } : {}),
            ...(text === undefined ? {} : { text }),
            type: call.object.typeReference,
            method: call.method,
            ...(target === undefined ? {} : { target }),
        });
    });

    return new NotificationType(owner, declared, described);
}

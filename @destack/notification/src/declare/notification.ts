import { ModuleMetadata } from "@destack/package";
import type { ObjectType } from "@destack/object";
import { schema } from "@destack/schema";
import { Action, type ActionDefinition } from "../notification/content.ts";
import {
    NotificationMetadata,
    NotificationType,
    type NotificationDefinition,
} from "../notification/notification.ts";

/** Declare a notification. */
export function defineNotification<
    Payload extends schema.Schema,
    const Types extends readonly ObjectType[] = [],
>(
    definition: NotificationDefinition<Payload, Types>,
    module?: ModuleMetadata,
): NotificationType<Payload> {
    // stamp the declaring package
    const owner = ModuleMetadata.require(module, "defineNotification").package;

    // validate the metadata
    const { actions = [], ...declared } = definition;
    const { name, title, description, interruption, preference } = declared;
    NotificationMetadata.parse({ name, title, description, interruption, preference });

    // refer to each action's type, requiring a mutating method of the declaring package
    const described = actions.map((action: ActionDefinition<schema.Infer<Payload>>) => {
        const { type, method, ...metadata } = action;
        // refuse another package's type
        if (type.package.id !== owner.id) {
            throw new TypeError(
                `action ${action.name} of ${name} calls ${type.name} of another package`,
            );
        }
        // refuse a method changing nothing
        else if (!type.method(method).mutates) {
            throw new TypeError(
                `action ${action.name} of ${name} calls ${type.name}.${method}, which changes nothing`,
            );
        }

        return Action.parse({ ...metadata, type: type.typeReference, method });
    });

    return new NotificationType(owner, declared, described);
}

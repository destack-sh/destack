import { type Localization, Message } from "@destack/locale";
import type { Action } from "@destack/object";
import { DeclarationName } from "@destack/package";
import { type JsonValue, schema } from "@destack/schema";
import { ObjectTypeReference } from "@destack/sync";

/** The content every channel shows, rendered in its recipient's locale. */
export const NotificationContent = schema.object({
    /** The headline. */
    title: schema.string(),
    /** The line below the headline. */
    subtitle: schema.string().exactOptional(),
    /** The text. */
    body: schema.string(),
});
/** The content every channel shows, rendered in its recipient's locale. */
export type NotificationContent = schema.Infer<typeof NotificationContent>;

/** The content a declaration writes: messages whose arguments are the payload's fields. */
export interface NotificationContentDefinition<Names extends string = string> {
    /** The headline. */
    readonly title: Message<Names>;
    /** The line below the headline. */
    readonly subtitle?: Message<Names>;
    /** The text. */
    readonly body: Message<Names>;
}

/** The content a declaration writes, as its build describes it. */
export const NotificationContentDefinition = Object.assign(
    schema.object({
        /** The headline. */
        title: Message,
        /** The line below the headline. */
        subtitle: Message.exactOptional(),
        /** The text. */
        body: Message,
    }),
    {
        /** Render the messages in a recipient's locale, their arguments filled from a payload. */
        render(
            definition: NotificationContentDefinition,
            payload: Readonly<Record<string, JsonValue>>,
            locale: Localization,
        ): NotificationContent {
            const write = (message: Message) => locale.render(message, payload);

            return {
                title: write(definition.title),
                ...(definition.subtitle === undefined
                    ? {}
                    : { subtitle: write(definition.subtitle) }),
                body: write(definition.body),
            };
        },
    },
);

/** The text field a notification's action asks for, as Apple's text input actions do. */
const NotificationActionText = schema.object({
    /** The field's placeholder. */
    placeholder: schema.string().min(1),
    /** The send button's label. */
    button: schema.string().min(1),
});

/** A notification's action as its recipient sees it, as the Web Notifications API's: its button. */
export const NotificationAction = schema.object({
    /** The declared action's name. */
    name: DeclarationName,
    /** The button's label, in the recipient's language. */
    title: schema.string().min(1),
    /** Whether the action removes its target for good. */
    isDestructive: schema.boolean().exactOptional(),
    /** The text field it asks for. */
    text: NotificationActionText.exactOptional(),
});
/** A notification's action as its recipient sees it. */
export type NotificationAction = schema.Infer<typeof NotificationAction>;

/** A notification's action as its build describes it: its button and the method it calls. */
export const NotificationActionDescription = NotificationAction.extend({
    /** The button's label. */
    title: Message,
    /** The type whose method it calls, of the declaring package. */
    type: ObjectTypeReference,
    /** The mutating method it calls, given the recipient's text as `text` where it asks. */
    method: schema.string().min(1),
    /** The payload field identifying the object it calls, the source when absent. */
    target: schema.string().min(1).exactOptional(),
});
/** A notification's action as its build describes it. */
export type NotificationActionDescription = schema.Infer<typeof NotificationActionDescription>;

/** The payload fields with string values, which may identify an action's target. */
type StringField<Payload> = {
    [Field in keyof Payload]: Payload[Field] extends string ? Field : never;
}[keyof Payload] &
    string;

/** An action beside a notification, as its package defines it: a declared action it offers. */
export interface NotificationActionDefinition<Payload> {
    /** The declared action, which calls a mutating method of the declaring package. */
    readonly action: Action;
    /** The payload field identifying the object it calls, the source when absent. */
    readonly target?: StringField<Payload>;
    /** The text field it asks for, given to the method as `text`. */
    readonly text?: schema.Infer<typeof NotificationActionText>;
}

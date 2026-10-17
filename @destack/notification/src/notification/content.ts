import { type Localization, Message } from "@destack/locale";
import type { MutationName, ObjectType } from "@destack/object";
import { defineSchema, type JsonValue, schema } from "@destack/schema";
import { ObjectTypeReference } from "@destack/sync";

/** An action's name, in camel case, as notification actions' identifiers are. */
export const ActionName = defineSchema(schema.string().regex(/^[a-z][a-zA-Z0-9]*$/u));

/** The content every channel shows, rendered in its recipient's locale. */
export const NotificationContent = defineSchema(
    schema.object({
        /** The headline. */
        title: schema.string(),
        /** The line below the headline. */
        subtitle: schema.string().exactOptional(),
        /** The text. */
        body: schema.string(),
    }),
);
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
    defineSchema(
        schema.object({
            /** The headline. */
            title: Message,
            /** The line below the headline. */
            subtitle: Message.exactOptional(),
            /** The text. */
            body: Message,
        }),
    ),
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

/** An action's metadata: what its button shows. */
export const ActionMetadata = defineSchema(
    schema.object({
        /** The name, in camel case. */
        name: ActionName,
        /** The button's label. */
        title: schema.string().min(1),
        /** Whether the action destroys or declines. */
        isDestructive: schema.boolean().exactOptional(),
        /** The text field it asks for. */
        text: schema
            .object({
                /** The field's placeholder. */
                placeholder: schema.string().min(1),
                /** The send button's label. */
                button: schema.string().min(1),
            })
            .exactOptional(),
    }),
);
/** An action's metadata. */
export type ActionMetadata = schema.Infer<typeof ActionMetadata>;

/** An action as its build describes it: its button, and the method of its package it calls. */
export const Action = defineSchema(
    ActionMetadata.extend({
        /** The type whose method it calls, of the declaring package. */
        type: ObjectTypeReference,
        /** The mutating method it calls, given the recipient's text as `text` where it asks. */
        method: schema.string().min(1),
        /** The payload field identifying the object it calls, the source when absent. */
        target: schema.string().min(1).exactOptional(),
    }),
);
/** An action as its build describes it. */
export type Action = schema.Infer<typeof Action>;

/** The payload fields with string values, which may identify an action's target. */
type StringField<Payload> = {
    [Field in keyof Payload]: Payload[Field] extends string ? Field : never;
}[keyof Payload] &
    string;

/** An action beside a notification: a call of its package's method as the recipient. */
export interface ActionDefinition<
    Payload,
    Type extends ObjectType = ObjectType,
> extends ActionMetadata {
    /** The type whose method it calls, of the declaring package. */
    readonly type: Type;
    /** The mutating method it calls, given the recipient's text as `text` where it asks. */
    readonly method: ObjectType extends Type ? string : MutationName<Type>;
    /** The payload field identifying the object it calls, the source when absent. */
    readonly target?: StringField<Payload>;
}

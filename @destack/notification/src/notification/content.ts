import type { Localization, Message } from "@destack/locale";
import type { Call } from "@destack/object";
import { defineSchema, schema } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";

/** The text every channel shows, rendered in its recipient's locale. */
export const Content = defineSchema(
    schema.object({
        /** The headline. */
        title: schema.string(),
        /** The line below the headline. */
        subtitle: schema.string().exactOptional(),
        /** The text. */
        body: schema.string(),
    }),
);
/** The text every channel shows, rendered in its recipient's locale. */
export type Content = schema.Infer<typeof Content>;

/** A text as a declaration writes it: as it reads, or a message rendered in each recipient's locale. */
type Text = string | Message;

/** The text every channel shows as a declaration writes it, its messages rendered in each recipient's locale. */
export interface ContentDefinition {
    /** The headline. */
    readonly title: Text;
    /** The line below the headline. */
    readonly subtitle?: Text;
    /** The text. */
    readonly body: Text;
}

/** The text a declaration writes, rendered in a recipient's locale. */
export const ContentDefinition = {
    /** Render a definition's messages in a recipient's locale. */
    render(definition: ContentDefinition, locale: Localization): Content {
        const write = (text: Text) => (typeof text === "string" ? text : locale.render(text));

        return {
            title: write(definition.title),
            ...(definition.subtitle === undefined ? {} : { subtitle: write(definition.subtitle) }),
            body: write(definition.body),
        };
    },
};

/** An action's metadata: what its button shows. */
export const ActionMetadata = defineSchema(
    schema.object({
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

/** An action beside a notification: one effect the recipient makes. */
export interface Action<Payload = unknown> extends ActionMetadata {
    /** Make the action's effect as the recipient, in the call answering the activity. */
    effect(
        activity: { readonly source: ObjectReference; readonly payload: Payload },
        call: Call,
        text?: string,
    ): Promise<unknown>;
}

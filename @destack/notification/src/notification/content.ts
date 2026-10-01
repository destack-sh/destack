import type { Call } from "@destack/object";
import type { ObjectReference } from "@destack/sync";

/** The text every channel shows. */
export interface Content {
    /** The headline. */
    readonly title: string;
    /** The line below the headline. */
    readonly subtitle?: string;
    /** The text. */
    readonly body: string;
}

/** An action beside a notification: one effect the recipient makes. */
export interface Action<Payload = unknown> {
    /** The button's label. */
    readonly title: string;
    /** Whether the action destroys or declines. */
    readonly isDestructive?: boolean;
    /** The text field it asks for. */
    readonly text?: {
        /** The field's placeholder. */
        readonly placeholder: string;
        /** The send button's label. */
        readonly button: string;
    };
    /** Make the action's effect as the recipient, in the call answering the notification. */
    effect(
        notification: { readonly source: ObjectReference; readonly payload: Payload },
        call: Call,
        text?: string,
    ): Promise<unknown>;
}

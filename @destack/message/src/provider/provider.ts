import type { Channel, Message, MessageError } from "../object/index.ts";

/** What a provider made of one message: sent, refused for now and retried after a wait it may name in milliseconds, or refused for good. */
export type Outcome =
    | { readonly outcome: "sent" }
    | { readonly outcome: "retry"; readonly after?: number; readonly error: MessageError }
    | { readonly outcome: "failed"; readonly error: MessageError };

/** A provider sending the messages of one channel, such as SES for email or Standard Webhooks for webhooks. */
export interface MessageProvider {
    /** The channel it sends on. */
    readonly channel: Channel;
    /** Send one message, keyed by its identifier so a repeated send delivers it once. */
    send(
        message: Pick<Message, "id" | "createdAt" | "to" | "content" | "secret">,
    ): Promise<Outcome>;
}

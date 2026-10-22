import { schema } from "@destack/schema";

/** A channel to every party of one topic, as PostgreSQL's LISTEN and NOTIFY. */
export interface Channel<Message> {
    /** Send a message to every other party. */
    notify(message: Message): void;
    /** Receive other parties' messages until stopped, resuming per delivery start. */
    listen(
        receive: (message: Message) => void,
        resume?: () => void,
        fail?: (error: unknown) => void,
    ): () => void;
}

/** A commit announced on a database's channel. */
export const Commit = schema.object({
    /** The message kind. */
    kind: schema.literal("commit"),
    /** The SQL names of the written tables, every declared table when the writer cannot know. */
    tables: schema.array(schema.string()),
    /** The log sequence the commit ends at, absent where its writer announces it unnumbered. */
    sequence: schema.number().int().exactOptional(),
});
/** A commit announced on a database's channel. */
export type Commit = schema.Infer<typeof Commit>;

/** Type a channel's messages by a schema, failing delivery on a message it refuses. */
export function typedChannel<Message>(
    channel: Channel<unknown>,
    message: schema.Schema<Message>,
): Channel<Message> {
    return {
        notify: (value) => channel.notify(value),
        listen: (receive, resume, fail) =>
            channel.listen(
                (value) => {
                    // deliver a parsed message, and fail on one the schema refuses
                    const parsed = message.safeParse(value);
                    if (parsed.success) {
                        receive(parsed.data);
                    } else if (fail !== undefined) {
                        fail(parsed.error);
                    } else {
                        throw parsed.error;
                    }
                },
                resume,
                fail,
            ),
    };
}

/** Connect parties to an in-process hub, delivering each message after the current task. */
export function channelHub<Payload>(): () => Channel<Payload> {
    const listeners = new Set<(message: Payload) => void>();

    return () => {
        const local = new Set<(message: Payload) => void>();

        return {
            notify: (message) => {
                // deliver to every other party
                for (const listener of listeners) {
                    if (!local.has(listener)) {
                        queueMicrotask(() => listener(structuredClone(message)));
                    }
                }
            },
            listen: (receive, resume) => {
                // deliver each later message
                listeners.add(receive);
                local.add(receive);
                resume?.();

                return () => {
                    listeners.delete(receive);
                    local.delete(receive);
                };
            },
        };
    };
}

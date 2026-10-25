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
    /** Wait until every party listening in this process heard a commit with a token. */
    readonly hear: (token: string) => Hearing;
    /** Leave the channel, releasing what opening and joining it took. */
    readonly close: () => Promise<void>;
}

/** The wait for a commit's token to reach every party listening in a process. */
export interface Hearing {
    /** Settles once every listening party heard the token, or stopped listening. */
    readonly heard: Promise<void>;
    /** Stop waiting, for a transaction whose commit sends no token. */
    readonly forget: () => void;
}

/** Hear a token at once, where a channel delivers inside the process before the commit returns. */
export const HEARD: Hearing = { heard: Promise.resolve(), forget: () => {} };

/** A commit announced on a database's channel. */
export const Commit = schema.object({
    /** The message kind. */
    kind: schema.literal("commit"),
    /** The SQL names of the written tables, every declared table when the writer cannot know. */
    tables: schema.array(schema.string()),
    /** The log sequence the commit ends at, absent where its writer announces it unnumbered. */
    sequence: schema.number().int().exactOptional(),
    /** The token of a committed transaction, which its own connection waits to hear. */
    token: schema.string().exactOptional(),
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
        hear: (token) => channel.hear(token),
        close: () => channel.close(),
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
            hear: () => HEARD,
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
            close: async () => {
                // stop this party's listeners
                for (const receive of local) {
                    listeners.delete(receive);
                }
                local.clear();
            },
        };
    };
}

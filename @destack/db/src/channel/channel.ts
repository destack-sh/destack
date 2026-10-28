import { schema } from "@destack/schema";

/** A channel to every party of one topic, as PostgreSQL's LISTEN and NOTIFY. */
export interface Channel<Message> {
    /** Send a message to every other party. */
    notify(message: Message): void;
    /** Receive other parties' messages until stopped. */
    listen(listener: ChannelListener<Message>): () => void;
    /** Wait until a commit's token reaches every party listening in this process. */
    readonly delivery: (token: string) => Delivery;
    /** Leave the channel, releasing what opening and joining it took. */
    readonly close: () => Promise<void>;
}

/** A party listening on a channel: what it receives, and how delivery starts, ends and fails. */
export interface ChannelListener<Message> {
    /** Receive another party's message. */
    readonly receive: (message: Message) => void;
    /** Recheck once delivery starts or starts again. */
    readonly resume?: () => void;
    /** Fail once delivery fails. */
    readonly fail?: (error: unknown) => void;
    /** Stop trusting delivery once it is lost, until it resumes. */
    readonly pause?: () => void;
}

/** The wait for a commit's token to reach every party listening in a process. */
export interface Delivery {
    /** Settles once the token reached every listening party, or each stopped listening. */
    readonly delivered: Promise<void>;
    /** Stop waiting, for a transaction whose commit sends no token. */
    readonly forget: () => void;
}

/** Deliver a token at once, where the channel delivers in process before the commit returns. */
export const DELIVERED: Delivery = { delivered: Promise.resolve(), forget: () => {} };

/** A commit announced on a database's channel. */
export const Commit = schema.object({
    /** The message kind. */
    kind: schema.literal("commit"),
    /** The SQL names of the written tables, every declared table when the writer cannot know. */
    tables: schema.array(schema.string()),
    /** The log sequence the commit ends at, absent where its writer announces it unnumbered. */
    sequence: schema.number().int().exactOptional(),
    /** The token of a committed transaction, whose delivery its own connection waits for. */
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
        delivery: (token) => channel.delivery(token),
        close: () => channel.close(),
        listen: (listener) =>
            channel.listen({
                ...listener,
                receive: (value) => {
                    // deliver a parsed message, and fail on one the schema refuses
                    const parsed = message.safeParse(value);
                    if (parsed.success) {
                        listener.receive(parsed.data);
                    } else if (listener.fail !== undefined) {
                        listener.fail(parsed.error);
                    } else {
                        throw parsed.error;
                    }
                },
            }),
    };
}

/** Connect parties to an in-process hub, delivering each message after the current task. */
export function channelHub<Payload>(): () => Channel<Payload> {
    const listeners = new Set<(message: Payload) => void>();

    return () => {
        const local = new Set<(message: Payload) => void>();

        return {
            delivery: () => DELIVERED,
            notify: (message) => {
                // deliver to every other party
                for (const listener of listeners) {
                    if (!local.has(listener)) {
                        queueMicrotask(() => listener(structuredClone(message)));
                    }
                }
            },
            listen: ({ receive, resume }) => {
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

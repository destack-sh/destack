import { schema } from "@destack/schema";
import { DatabaseError } from "../error/error.ts";

/** A channel reaching every party of one topic, as PostgreSQL's LISTEN and NOTIFY. */
export interface Channel<Message> {
    /** Send a message to every other party. */
    notify(message: Message): void;
    /** Receive the other parties' messages until stopped, resuming on each delivery start and failing once delivery ends. */
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
    /** The SQL names of the tables the commit wrote, every declared table when the writer cannot know them. */
    tables: schema.array(schema.string()),
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

/** Reach every party of a channel within one origin, such as the tabs and workers of a browser. */
export function broadcastChannel(name: string): Channel<unknown> {
    const broadcast = new BroadcastChannel(name);

    return {
        notify: (message) => broadcast.postMessage(message),
        listen: (receive, resume, fail) => {
            // deliver each later message, and fail on one the browser cannot read
            const listener = (event: Event) => {
                if (event instanceof MessageEvent) {
                    const data: unknown = event.data;
                    receive(data);
                }
            };
            const refused = () =>
                fail?.(new DatabaseError("QUERY_FAILED", "a channel message could not be read"));
            broadcast.addEventListener("message", listener);
            broadcast.addEventListener("messageerror", refused);
            resume?.();

            return () => {
                broadcast.removeEventListener("message", listener);
                broadcast.removeEventListener("messageerror", refused);
            };
        },
    };
}

/** Connect parties to an in-process hub, delivering each message after the current task, such as the instances one process runs. */
export function channelHub<Payload>(): () => Channel<Payload> {
    const listeners = new Set<(message: Payload) => void>();

    return () => {
        const own = new Set<(message: Payload) => void>();

        return {
            notify: (message) => {
                // deliver to every other party
                for (const listener of listeners) {
                    if (!own.has(listener)) {
                        queueMicrotask(() => listener(structuredClone(message)));
                    }
                }
            },
            listen: (receive, resume) => {
                // deliver each later message
                listeners.add(receive);
                own.add(receive);
                resume?.();

                return () => {
                    listeners.delete(receive);
                    own.delete(receive);
                };
            },
        };
    };
}

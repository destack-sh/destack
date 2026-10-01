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

/** Open a database's channel of a name, reaching the other connections to the same database. */
export type OpenChannel = <Message>(name: string) => Channel<Message>;

/** A commit announced on a database's channel. */
export interface Commit {
    /** The message kind. */
    readonly kind: "commit";
}

/** Reach every party of a channel within one origin, such as the tabs and workers of a browser. */
export function broadcastChannel<Message>(name: string): Channel<Message> {
    const broadcast = new BroadcastChannel(name);

    return {
        notify: (message) => broadcast.postMessage(message),
        listen: (receive, resume) => {
            // deliver each later message
            const listener = (event: Event) => receive((event as MessageEvent<Message>).data);
            broadcast.addEventListener("message", listener);
            resume?.();

            return () => broadcast.removeEventListener("message", listener);
        },
    };
}

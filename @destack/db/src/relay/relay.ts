/** A relay every party of one topic reaches, such as the tabs of one origin or the instances sharing a database. */
export interface Relay<Payload> {
    /** Send a message to every other party. */
    post(message: Payload): void;
    /** Receive the other parties' messages until the returned stop runs, resuming whenever delivery starts or restarts. */
    listen(receive: (message: Payload) => void, resume?: () => void): () => void;
}

/** Reach every party of a broadcast relay, such as the tabs and workers of one origin. */
export function broadcastRelay<Payload>(name: string): Relay<Payload> {
    const broadcast = new BroadcastChannel(name);

    return {
        post: (message) => broadcast.postMessage(message),
        listen: (receive, resume) => {
            // deliver each later message, starting now
            const listener = (event: Event) => receive((event as MessageEvent<Payload>).data);
            broadcast.addEventListener("message", listener);
            resume?.();

            return () => broadcast.removeEventListener("message", listener);
        },
    };
}

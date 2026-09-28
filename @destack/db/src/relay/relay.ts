/** A channel reaching every party of one topic. */
export interface Relay<Payload> {
    /** Send a message to every other party. */
    post(message: Payload): void;
    /** Receive the other parties' messages until stopped, resuming on each delivery start. */
    listen(receive: (message: Payload) => void, resume?: () => void): () => void;
}

/** Reach every party of a broadcast channel. */
export function broadcastRelay<Payload>(name: string): Relay<Payload> {
    const broadcast = new BroadcastChannel(name);

    return {
        post: (message) => broadcast.postMessage(message),
        listen: (receive, resume) => {
            // deliver each later message
            const listener = (event: Event) => receive((event as MessageEvent<Payload>).data);
            broadcast.addEventListener("message", listener);
            resume?.();

            return () => broadcast.removeEventListener("message", listener);
        },
    };
}

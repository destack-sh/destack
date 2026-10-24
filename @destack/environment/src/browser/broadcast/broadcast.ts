/** A named channel between the origin's tabs and workers, as BroadcastChannel has it. */
export interface BroadcastPort {
    /** Send a message to every other port of the channel. */
    readonly send: (message: unknown) => void;
    /** Call a listener on each message from another port until the subscription disposes. */
    readonly on: (listener: (message: unknown) => void) => Disposable;
    /** Close the port. */
    readonly close: () => void;
}

/** The origin's broadcast channels. */
export interface Broadcast {
    /** Open a port of a named channel. */
    readonly open: (name: string) => BroadcastPort;
}

/** Broadcast channel sources. */
export const Broadcast: { readonly system: Broadcast } = {
    /** The browser's broadcast channels. */
    system: {
        open: (name) => {
            const channel = new BroadcastChannel(name);

            return {
                send: (message) => channel.postMessage(message),
                on: (listener) => {
                    const receive = (event: MessageEvent) => listener(event.data);
                    channel.addEventListener("message", receive);

                    return {
                        [Symbol.dispose]: () => channel.removeEventListener("message", receive),
                    };
                },
                close: () => channel.close(),
            };
        },
    },
};

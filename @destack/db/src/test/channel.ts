import type { Channel } from "../channel/channel.ts";

/** Connect parties to one in-process hub, delivering each message to every other party after the current task, as a broadcast channel does. */
export function channelHub<Payload>(): () => Channel<Payload> {
    const listeners = new Set<(message: Payload) => void>();

    return () => {
        const own = new Set<(message: Payload) => void>();

        return {
            post: (message) => {
                // deliver a copy to every other party's listeners
                for (const listener of listeners) {
                    if (!own.has(listener)) {
                        queueMicrotask(() => listener(structuredClone(message)));
                    }
                }
            },
            listen: (receive, resume) => {
                // deliver each later message, starting now
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

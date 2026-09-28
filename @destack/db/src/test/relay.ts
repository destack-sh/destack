import type { Relay } from "../relay/relay.ts";

/** Connect parties to an in-process hub, delivering each message after the current task. */
export function relayHub<Payload>(): () => Relay<Payload> {
    const listeners = new Set<(message: Payload) => void>();

    return () => {
        const own = new Set<(message: Payload) => void>();

        return {
            post: (message) => {
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

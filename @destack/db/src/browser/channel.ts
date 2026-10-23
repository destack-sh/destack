import type { Channel } from "../channel/index.ts";
import { DatabaseError } from "../error/error.ts";

/** Send to every party of a channel within one origin, such as a browser's tabs and workers. */
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
                fail?.(DatabaseError.of("QUERY_FAILED", "cannot read a channel message"));
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

import type { Environment } from "@destack/environment";
import type {} from "@destack/environment/browser";
import { type Channel, DELIVERED } from "../channel/index.ts";

/** Send to every party of a channel within one origin, such as a browser's tabs and workers. */
export function broadcastChannel(
    name: string,
    environment: Pick<Environment<"broadcast">, "broadcast">,
): Channel<unknown> {
    const port = environment.broadcast.open(name);

    return {
        notify: (message) => port.send(message),
        delivery: () => DELIVERED,
        listen: ({ receive, resume }) => {
            // deliver each later message
            const subscription = port.on(receive);
            resume?.();

            return () => subscription[Symbol.dispose]();
        },
        close: async () => port.close(),
    };
}

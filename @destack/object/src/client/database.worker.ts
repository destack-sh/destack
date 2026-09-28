import { broadcastRelay } from "@destack/db/relay";
import type { Message } from "@destack/db/shared";
import { serveBrowserDatabase } from "@destack/db/wasm";

// serve the named database to every tab on its relay
self.addEventListener(
    "message",
    (event: MessageEvent<{ readonly name: string }>) => {
        const name = event.data.name;
        void serveBrowserDatabase(name, broadcastRelay<Message>(`destack:${name}`));
    },
    { once: true },
);

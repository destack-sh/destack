import { broadcastChannel } from "@destack/db/channel";
import type { Message } from "@destack/db/shared";
import { serveBrowserDatabase } from "@destack/db/wasm";

// serve the named database to every tab on its channel
self.addEventListener(
    "message",
    (event: MessageEvent<{ readonly name: string }>) => {
        const name = event.data.name;
        void serveBrowserDatabase(name, broadcastChannel<Message>(`destack:${name}`));
    },
    { once: true },
);

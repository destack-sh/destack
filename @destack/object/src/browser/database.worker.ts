import { broadcastChannel } from "@destack/db";
import { serveBrowserDatabase } from "@destack/db/browser";

// serve the named database to every tab on its channel
self.addEventListener(
    "message",
    (event: MessageEvent<{ readonly name: string }>) => {
        const name = event.data.name;
        void serveBrowserDatabase(name, broadcastChannel(`destack:${name}`));
    },
    { once: true },
);

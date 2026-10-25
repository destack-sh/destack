import { broadcastChannel, serveBrowserDatabase } from "@destack/db/browser";
import { BrowserEnvironment } from "@destack/environment/browser";

// serve the requested database to every tab on its channel, in the worker's own environment
self.addEventListener(
    "message",
    (event: MessageEvent<{ readonly name: string }>) => {
        const name = event.data.name;
        const channel = broadcastChannel(`destack:${name}`, BrowserEnvironment.system);
        void serveBrowserDatabase(name, channel, BrowserEnvironment.system);
    },
    { once: true },
);

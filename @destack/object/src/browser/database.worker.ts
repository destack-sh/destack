import { broadcastChannel, serveBrowserDatabase } from "@destack/db/browser";
import { BrowserEnvironment } from "@destack/environment/browser";

// serve the requested database to every tab on its channel, telling the starting tab it serves or failed
self.addEventListener(
    "message",
    (event: MessageEvent<{ readonly name: string }>) => {
        const name = event.data.name;
        const channel = broadcastChannel(`destack:${name}`, BrowserEnvironment.system);
        serveBrowserDatabase(name, channel, BrowserEnvironment.system).then(
            () => self.postMessage({ kind: "serving" }),
            (error: unknown) =>
                self.postMessage({
                    kind: "failed",
                    message:
                        error instanceof Error ? `${error.name}: ${error.message}` : String(error),
                }),
        );
    },
    { once: true },
);

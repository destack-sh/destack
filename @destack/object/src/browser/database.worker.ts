// install the web-standard APIs the Destack browser baseline lacks before any other module runs
import "@destack/environment/baseline";

// serve the requested database to every tab on its channel, telling the starting tab it serves or failed
self.addEventListener(
    "message",
    (event: MessageEvent<{ readonly name: string }>) => {
        // load the database after the baseline, reporting a failed load as the worker's own error
        const name = event.data.name;
        Promise.all([import("@destack/db/browser"), import("@destack/environment/browser")]).then(
            ([{ broadcastChannel, serveBrowserDatabase }, { BrowserEnvironment }]) => {
                // serve it on its channel, telling the starting tab it serves or failed
                const channel = broadcastChannel(`destack:${name}`, BrowserEnvironment.system);
                serveBrowserDatabase(name, channel, BrowserEnvironment.system).then(
                    () => self.postMessage({ kind: "serving" }),
                    (error: unknown) =>
                        self.postMessage({
                            kind: "failed",
                            message:
                                error instanceof Error
                                    ? `${error.name}: ${error.message}`
                                    : String(error),
                        }),
                );
            },
            (error: unknown) => self.reportError(error),
        );
    },
    { once: true },
);

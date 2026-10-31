import type { BrowserEnvironment } from "@destack/environment/browser";
import type { TelemetryScope } from "@destack/telemetry";

/** Record a visit of a view as its page loads and each time it moves to another path. */
export function recordVisits(
    view: string,
    telemetry: TelemetryScope,
    environment: Pick<BrowserEnvironment, "navigation" | "locale">,
    signal: AbortSignal,
): void {
    // record a visit of the current path, with the host the visitor came from
    const { navigation } = environment;
    let visitedPath = "";
    const visited = (referrer?: string) => {
        visitedPath = new URL(navigation.currentEntry.url).pathname;
        telemetry.visit(view, {
            path: visitedPath,
            title: document.title,
            locale: environment.locale,
            ...(referrer === undefined ? {} : { referrer }),
        });
    };
    visited(document.referrer === "" ? undefined : new URL(document.referrer).host);

    // record each move to another path until the signal aborts
    const following = navigation.on((entry) => {
        if (new URL(entry.url).pathname !== visitedPath) {
            visited();
        }
    });
    signal.addEventListener("abort", () => following[Symbol.dispose](), { once: true });
}

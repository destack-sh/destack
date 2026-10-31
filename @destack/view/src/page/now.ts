import { type Accessor, createSignal, onCleanup, sharedConfig } from "solid-js";
import { isServer } from "@solidjs/web";
import { Duration } from "@destack/schema";
import { useEnvironment } from "./context.ts";

/** How often the time a view reads moves on: each second, finer than any relative time it shows. */
const NOW_INTERVAL: Duration = { seconds: 1 };

/** Read the time on the view's clock once mounted, absent before, moving on at each interval. */
export function useNow(interval: Duration = NOW_INTERVAL): Accessor<number | undefined> {
    // read no time on the server and while hydrating, so both render the same, else the clock's
    const { clock } = useEnvironment();
    const isDeferred = isServer || sharedConfig.hydrating;
    const [now, setNow] = createSignal<number | undefined>(isDeferred ? undefined : clock.now(), {
        ownedWrite: true,
    });
    if (isServer) {
        return now;
    }

    // read a deferred time a task after mounting, once hydration completed, then at each interval
    const first = isDeferred ? clock.setTimeout(() => setNow(clock.now()), 0) : undefined;
    const handle = clock.setInterval(() => setNow(clock.now()), Duration.milliseconds(interval));
    onCleanup(() => {
        clock.clearTimeout(first);
        clock.clearInterval(handle);
    });

    return now;
}

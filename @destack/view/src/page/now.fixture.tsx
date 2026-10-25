import { useNow } from "./view.ts";

/** Show the view's time once mounted, and a placeholder before. */
export function CurrentTime() {
    // read the time, absent on the server and while hydrating
    const now = useNow();

    return <p>time {now() ?? "unknown"}</p>;
}

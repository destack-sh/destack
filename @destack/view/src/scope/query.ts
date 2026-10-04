import type { RelationalQuery } from "@destack/object/client";
import { type Accessor, createMemo, onCleanup } from "../solid/reactive.ts";

/** Follow a relational query, pending until the scope's copy holds it and failing into the nearest error boundary. */
export function useQuery<Value>(
    query: RelationalQuery<Value> | Accessor<RelationalQuery<Value>>,
): Accessor<Value> {
    return createMemo(() => {
        // follow the current query until it changes or the component goes
        const current = typeof query === "function" ? query() : query;
        const stopping = new AbortController();
        onCleanup(() => stopping.abort());

        return follow(current, stopping.signal);
    });
}

/** Follow one query until the signal aborts. */
async function* follow<Value>(
    query: RelationalQuery<Value>,
    signal: AbortSignal,
): AsyncGenerator<Value> {
    const live = query.subscribe();
    try {
        for await (const value of live.watch(signal)) {
            yield value;
        }
    } finally {
        await live.close();
    }
}

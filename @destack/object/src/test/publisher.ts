import type { Feed, Publisher, Query, Subscription } from "@destack/sync";
import type { ObjectRemote } from "../replica/remote.ts";

/** A home's feed published to followers in process. */
export const FeedPublisher = {
    /** Publish a feed's pages of each subscription's queries. */
    of(
        feed: Feed,
        queries: (subscription: Subscription) => Readonly<Record<string, Query>>,
    ): Publisher {
        return {
            // stream the queries from the follower's position
            stream: (subscription, signal) =>
                feed.subscribe(queries(subscription), subscription.after, signal),
            // record nothing, as an in-process follower's stream follows its own queries
            request: async () => {},
        };
    },
};

/** A remote in tests: a publisher's pages, and the writes a test takes, refusing the rest. */
export const TestRemote = {
    /** Stream a publisher's pages, and take the given writes. */
    of(
        publisher: Publisher,
        writes: Partial<Pick<ObjectRemote, "push" | "call" | "broadcast">> = {},
    ): ObjectRemote {
        return {
            stream: (subscription, signal) => publisher.stream(subscription, signal),
            request: (subscription) => publisher.request(subscription),
            push: writes.push ?? TestRemote.refused("pushes"),
            call: writes.call ?? TestRemote.refused("calls"),
            broadcast: writes.broadcast ?? TestRemote.refused("events"),
        };
    },

    /** Refuse a kind of write the test takes none of. */
    refused(writes: string): () => Promise<never> {
        return () =>
            Promise.reject(new Error(`'${writes}' are refused: the test remote takes none`));
    },
};

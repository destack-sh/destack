import type { Feed, Publisher, Query, Subscription } from "@destack/sync";

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

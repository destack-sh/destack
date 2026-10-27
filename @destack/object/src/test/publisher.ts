import type { Feed, Publisher, Query, Stream, Subscription } from "@destack/sync";

/** A home's feed published to followers in process, confirming their streams as they ask. */
export const FeedPublisher = {
    /** Publish a feed's pages of each subscription's queries, confirming its streams. */
    of(
        feed: Feed,
        queries: (subscription: Subscription) => Readonly<Record<string, Query>>,
    ): Publisher {
        // store each subscription's open stream by its replica and target
        const streams = new Map<string, Stream>();

        return {
            // stream the queries, asking a new stream for the position its follower awaits
            stream: (subscription, signal) =>
                feed.subscribe(queries(subscription), subscription.after, signal, {
                    opened: (stream) => {
                        streams.set(FeedPublisher.key(subscription), stream);
                        if (subscription.confirming !== undefined) {
                            stream.confirm(subscription.confirming);
                        }
                    },
                    closed: () => streams.delete(FeedPublisher.key(subscription)),
                }),
            // ask a subscription's open stream for the watermark it requests
            request: async (subscription) => {
                const { confirming } = subscription;
                if (confirming !== undefined) {
                    streams.get(FeedPublisher.key(subscription))?.confirm(confirming);
                }
            },
        };
    },

    /** Key a subscription's stream by its replica and target. */
    key(subscription: Pick<Subscription, "name" | "scope" | "target">): string {
        return `${subscription.name} ${subscription.scope} ${subscription.target}`;
    },
};

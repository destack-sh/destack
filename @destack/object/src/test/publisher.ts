import type { Feed, Publisher, Query, Stream, Subscription } from "@destack/sync";
import { ObjectSource } from "../server/source.ts";

/** A home's feed published to followers in process, confirming their streams as they ask. */
export const FeedPublisher = {
    /** Publish a feed's pages of each subscription's queries, confirming its streams. */
    of(
        feed: Feed,
        queries: (subscription: Subscription) => Readonly<Record<string, Query>>,
    ): Publisher {
        // keep each subscription's open stream by its replica and target
        const streams = new Map<string, Stream>();

        return {
            // stream the queries, asking a new stream for the position its follower awaits
            stream: (subscription, signal) =>
                feed.subscribe(queries(subscription), subscription.after, signal, {
                    opened: (stream) => {
                        streams.set(ObjectSource.streamKey(subscription), stream);
                        if (subscription.confirming !== undefined) {
                            stream.confirm(subscription.confirming);
                        }
                    },
                    closed: () => streams.delete(ObjectSource.streamKey(subscription)),
                }),
            confirm: async (subscription, watermark) => {
                streams.get(ObjectSource.streamKey(subscription))?.confirm(watermark);
            },
        };
    },
};

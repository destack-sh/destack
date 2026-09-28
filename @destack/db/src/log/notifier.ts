import type { CommitWatch } from "./watch.ts";
import type { Relay } from "../relay/relay.ts";

/** A channel of commit notifications between the writers of one database. */
export interface CommitNotifier {
    /** Wake the readers on other writers' commits, until stopped. */
    listen(commits: CommitWatch): () => Promise<void>;
    /** Notify the other writers of a commit. */
    notify(): void;
}

/** A notifier for a database only its own connection writes. */
export const soleWriter: CommitNotifier = {
    listen: () => async () => {},
    notify: () => {},
};

/** Exchange commit notifications through a relay. */
export function relayNotifier(relay: Relay<{ readonly kind: string }>): CommitNotifier {
    return {
        listen(commits) {
            // wake readers on each commit and on each resumed delivery
            const stop = relay.listen(
                (message) => {
                    if (message.kind === "commit") {
                        commits.wake();
                    }
                },
                () => commits.wake(),
            );

            return async () => stop();
        },
        notify() {
            relay.post({ kind: "commit" });
        },
    };
}

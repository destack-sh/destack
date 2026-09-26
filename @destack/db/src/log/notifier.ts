import type { CommitWatch } from "./watch.ts";
import type { Log } from "./log.ts";
import type { Channel } from "../channel/channel.ts";

/** Carries commit notifications between the writers of one database, such as processes, tabs or servers. */
export interface CommitNotifier {
    /** Wake the watching readers on every commit another writer makes, until the returned stop runs. */
    listen(commits: CommitWatch, log: Log): () => Promise<void>;
    /** Notify the other writers of a commit this connection made. */
    notify(): void;
}

/** Notice other writers' commits by reading the latest sequence at an interval while readers wait. */
export function pollNotifier(interval: number): CommitNotifier {
    return {
        listen(commits, log) {
            // read the latest sequence only while readers wait
            let seen: number | undefined;
            let isReading = false;
            const timer = setInterval(async () => {
                if (!commits.isWaiting || isReading) {
                    return;
                }
                isReading = true;
                try {
                    // wake the readers when the sequence moves, the first read included
                    const latest = await log.latest();
                    if (latest !== seen) {
                        commits.wake();
                    }
                    seen = latest;
                } catch {
                    // wake the readers, whose own reads report the failure
                    commits.wake();
                } finally {
                    isReading = false;
                }
            }, interval);

            return async () => clearInterval(timer);
        },
        notify() {
            // leave the commit to the other writers' next poll
        },
    };
}

/** Notify the other parties of a channel of this party's commits, and listen for theirs. */
export function channelNotifier(channel: Channel<{ readonly kind: string }>): CommitNotifier {
    return {
        listen(commits) {
            // wake readers on each commit, and whenever delivery resumes, since commits before it went unannounced
            const stop = channel.listen(
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
            channel.post({ kind: "commit" });
        },
    };
}

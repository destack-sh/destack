import { wait } from "./timer.ts";

/** How a failed operation retries, as Temporal's retry policy. */
export interface RetryPolicy {
    /** The wait before the second attempt, in milliseconds. */
    readonly initialInterval: number;
    /** The growth factor of each wait, 1 for a constant wait. */
    readonly backoffCoefficient: number;
    /** The longest wait between attempts, in milliseconds. */
    readonly maximumInterval: number;
    /** The most attempts, the first included, absent to retry until stopped. */
    readonly maximumAttempts?: number;
    /** Spread each wait uniformly below its interval, as AWS's full jitter does. */
    readonly jitter?: "full";
}

/** Temporal's default policy: a second, doubling, at most 100 seconds apart. */
const DEFAULT_POLICY: RetryPolicy = {
    initialInterval: 1000,
    backoffCoefficient: 2,
    maximumInterval: 100_000,
};

/** Retry waits of failing operations. */
export const RetryPolicy = {
    /** Wait before the next attempt, resolving false once the signal aborts. */
    async pause(policy: RetryPolicy, failures: number, signal: AbortSignal): Promise<boolean> {
        return wait(RetryPolicy.interval(policy, failures), { signal }).then(
            () => true,
            (error: unknown) => {
                // end on abort and keep other failures
                if (!signal.aborted) {
                    throw error;
                }

                return false;
            },
        );
    },

    /** Complete a policy from the default. */
    of(changes: Partial<RetryPolicy> = {}): RetryPolicy {
        return { ...DEFAULT_POLICY, ...changes };
    },

    /** Read the wait before the next attempt, in milliseconds. */
    interval(policy: RetryPolicy, failures: number, random: () => number = Math.random): number {
        const grown = policy.initialInterval * policy.backoffCoefficient ** (failures - 1);
        const interval = Math.min(grown, policy.maximumInterval);

        return policy.jitter === "full" ? random() * interval : interval;
    },

    /** Decide whether an operation retries. */
    isRetried(policy: RetryPolicy, failures: number): boolean {
        return policy.maximumAttempts === undefined || failures < policy.maximumAttempts;
    },
};

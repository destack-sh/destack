import type { Clock } from "./clock.ts";

/** How a failed operation retries. */
export interface RetryPolicy {
    /** The wait before the second attempt, in milliseconds. */
    readonly initialInterval: number;
    /** The growth factor of each wait, 1 for a constant wait. */
    readonly backoffCoefficient: number;
    /** The longest wait between attempts, in milliseconds. */
    readonly maximumInterval: number;
    /** The most attempts, the first included, absent to retry until stopped. */
    readonly maximumAttempts?: number;
    /** Spread each wait uniformly below its interval, as full jitter. */
    readonly jitter?: "full";
}

/** The bits of a draw's hash, which a double holds exactly. */
const DRAW_BITS = 53;

/** The cyrb53 multipliers that spread a seed's characters over two 32-bit words. */
const DRAW_MULTIPLIERS = [2_654_435_761, 1_597_334_677, 2_246_822_507, 3_266_489_909] as const;

/** The default policy: a second, doubling, at most 100 seconds apart. */
const DEFAULT_POLICY: RetryPolicy = {
    initialInterval: 1000,
    backoffCoefficient: 2,
    maximumInterval: 100_000,
};

/** Retry waits of failing operations. */
export const RetryPolicy = {
    /** Wait on a clock before the next attempt, resolving false once the signal aborts. */
    async pause(
        policy: RetryPolicy,
        failures: number,
        clock: Clock,
        signal: AbortSignal,
        random: () => number,
    ): Promise<boolean> {
        return clock.sleep(RetryPolicy.interval(policy, failures, random), signal).then(
            () => true,
            (error: unknown) => {
                // end on abort and record other failures
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

    /** Read the wait before the next attempt, in whole milliseconds, rounded up. */
    interval(policy: RetryPolicy, failures: number, random: () => number): number {
        const grown = policy.initialInterval * policy.backoffCoefficient ** (failures - 1);
        const interval = Math.min(grown, policy.maximumInterval);

        return Math.ceil(policy.jitter === "full" ? random() * interval : interval);
    },

    /** Draw a fraction in [0, 1) from a seed: its 53-bit cyrb53 hash. */
    draw(seed: string): number {
        // mix each character into two words
        const [first, second, third, fourth] = DRAW_MULTIPLIERS;
        let high = 0xdeadbeef;
        let low = 0x41c6ce57;
        for (let index = 0; index < seed.length; index += 1) {
            const code = seed.charCodeAt(index);
            high = Math.imul(high ^ code, first);
            low = Math.imul(low ^ code, second);
        }

        // avalanche both words into each other
        high = Math.imul(high ^ (high >>> 16), third) ^ Math.imul(low ^ (low >>> 13), fourth);
        low = Math.imul(low ^ (low >>> 16), third) ^ Math.imul(high ^ (high >>> 13), fourth);
        const hash = 2 ** 32 * ((low >>> 0) & 0x1f_ffff) + (high >>> 0);

        return hash / 2 ** DRAW_BITS;
    },

    /** Decide whether an operation retries. */
    isRetried(policy: RetryPolicy, failures: number): boolean {
        return policy.maximumAttempts === undefined || failures < policy.maximumAttempts;
    },
};

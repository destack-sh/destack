import type { StepUp } from "@destack/access";

/** The share of a token's remaining life it is reused for. */
const REUSE_SHARE = 0.8;

/** A token exchanged for a caller, and when it lapses. */
export interface ExchangedToken {
    /** The access token. */
    readonly accessToken: string;
    /** When the token lapses, in UTC epoch milliseconds. */
    readonly expiresAt: number;
}

/** The tokens exchanged for callers. */
export class TokenCache {
    /** The exchanged tokens by their exchange's key. */
    readonly #tokens = new Map<string, CachedToken>();

    /** Reuse a key's token for 80% of its life while it answers the step-up, else exchange anew. */
    token(
        key: string,
        now: number,
        exchange: () => Promise<ExchangedToken>,
        stepUp?: StepUp,
    ): Promise<string> {
        // reuse a token within its window, exchanged at the assurance a step-up asks for and recently enough
        const cached = this.#tokens.get(key);
        const isAnswering =
            stepUp === undefined ||
            (cached !== undefined &&
                cached.assurance >= stepUp.assurance &&
                (stepUp.maxAge === undefined || now - cached.exchangedAt <= stepUp.maxAge));
        if (cached !== undefined && now < cached.until && isAnswering) {
            return cached.token;
        }

        // drop the tokens past their window
        for (const [entry, { until }] of this.#tokens) {
            if (until <= now) {
                this.#tokens.delete(entry);
            }
        }

        // exchange a new one, reused until 80% of its life passed, forgetting it once the exchange fails
        const exchanged = exchange();
        const entry: CachedToken = {
            token: exchanged.then((answer) => answer.accessToken),
            until: Infinity,
            assurance: stepUp?.assurance ?? 0,
            exchangedAt: now,
        };
        this.#tokens.set(key, entry);
        exchanged.then(
            (answer) => {
                entry.until = now + (answer.expiresAt - now) * REUSE_SHARE;
            },
            () => {
                if (this.#tokens.get(key) === entry) {
                    this.#tokens.delete(key);
                }
            },
        );

        return entry.token;
    }

    /** Forget every token, such as once the key that exchanged them changed. */
    clear(): void {
        this.#tokens.clear();
    }
}

/** A token the cache stores, with when it stops being reused and the step-up it answers. */
interface CachedToken {
    /** The exchanged access token. */
    readonly token: Promise<string>;
    /** When the token stops being reused, in UTC epoch milliseconds, never while it exchanges. */
    until: number;
    /** The assurance level the token was exchanged at. */
    readonly assurance: number;
    /** When the token was exchanged, in UTC epoch milliseconds. */
    readonly exchangedAt: number;
}

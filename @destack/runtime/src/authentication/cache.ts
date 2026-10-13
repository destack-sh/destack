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
    /** The exchanged tokens by their exchange's key, with the time each stops being reused. */
    readonly #tokens = new Map<string, { readonly token: Promise<string>; until: number }>();

    /** Reuse the token exchanged under a key for 80% of its life, or exchange a new one. */
    token(key: string, now: number, exchange: () => Promise<ExchangedToken>): Promise<string> {
        // reuse a token within its window
        const cached = this.#tokens.get(key);
        if (cached !== undefined && now < cached.until) {
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
        const entry = { token: exchanged.then((answer) => answer.accessToken), until: Infinity };
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
}

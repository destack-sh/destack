import { EXCHANGE_LIFETIME_MILLISECONDS } from "./token.ts";

/** How long an exchanged token is reused: 80% of its life. */
const REUSE_MILLISECONDS = EXCHANGE_LIFETIME_MILLISECONDS * 0.8;

/** The tokens exchanged for callers. */
export class TokenCache {
    /** The exchanged tokens by their exchange's key, with the time each stops being reused. */
    readonly #tokens = new Map<
        string,
        { readonly token: Promise<string>; readonly until: number }
    >();

    /** Reuse the token exchanged under a key within its reuse window, or exchange a new one. */
    token(key: string, now: number, exchange: () => Promise<string>): Promise<string> {
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

        // exchange a new one, forgetting it once the exchange fails
        const token = exchange();
        this.#tokens.set(key, { token, until: now + REUSE_MILLISECONDS });
        token.catch(() => {
            if (this.#tokens.get(key)?.token === token) {
                this.#tokens.delete(key);
            }
        });

        return token;
    }
}

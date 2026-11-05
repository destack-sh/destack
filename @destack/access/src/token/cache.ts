import { Jwt } from "@destack/identity";
import { present } from "@destack/schema";
import type { StepUp } from "../context/index.ts";

/** The milliseconds of a second, the unit of a JWT's times (RFC 7519 2). */
const SECOND_MILLISECONDS = 1000;

/** The share of a token's remaining life it is reused for. */
const REUSE_SHARE = 0.8;

/** The tokens exchanged for callers. */
export class TokenCache {
    /** The exchanged tokens by their exchange's key. */
    readonly #tokens = new Map<string, CachedToken>();

    /** Reuse a key's token for 80% of its life while it answers the step-up, else exchange anew. */
    token(
        key: string,
        now: number,
        exchange: () => Promise<string>,
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
            return cached.exchanged;
        }

        // drop the tokens past their window
        for (const [entry, { until }] of this.#tokens) {
            if (until <= now) {
                this.#tokens.delete(entry);
            }
        }

        // exchange a new one, reused until 80% of its life passed, forgetting it once the exchange fails
        //  NOTE #Suspicious: a step-up labels it with the asked assurance, which a refresh never raises
        const exchanged = exchange();
        const entry: CachedToken = {
            exchanged,
            until: Infinity,
            assurance: stepUp?.assurance ?? 0,
            exchangedAt: now,
        };
        this.#tokens.set(key, entry);
        exchanged.then(
            (token) => {
                const expiresAt =
                    present(Jwt.claims(token).exp, "the token's expiry") * SECOND_MILLISECONDS;
                entry.until = now + (expiresAt - now) * REUSE_SHARE;
            },
            () => {
                if (this.#tokens.get(key) === entry) {
                    this.#tokens.delete(key);
                }
            },
        );

        return exchanged;
    }

    /** Forget every token, such as once the key that exchanged them changed. */
    clear(): void {
        this.#tokens.clear();
    }
}

/** A token the cache stores, with when it stops being reused and the step-up it answers. */
interface CachedToken {
    /** The exchanged access token. */
    readonly exchanged: Promise<string>;
    /** When the token stops being reused, in UTC epoch milliseconds, never while it exchanges. */
    until: number;
    /** The assurance level the token was exchanged at. */
    readonly assurance: number;
    /** When the token was exchanged, in UTC epoch milliseconds. */
    readonly exchangedAt: number;
}

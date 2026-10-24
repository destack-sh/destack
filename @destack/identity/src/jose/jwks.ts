import type { Clock, Crypto } from "@destack/environment";
import { schema } from "@destack/schema";
import { IdentityError } from "../error/error.ts";
import { Jwk } from "./jwk.ts";
import { JwsAlgorithm } from "./algorithm.ts";
import { Jws, type JwsHeader, type JwsKey, type KeyLookup } from "./jws.ts";

/** A JSON Web Key Set (RFC 7517 5). */
export const JwkSet = schema.object({
    /** The keys. */
    keys: schema.array(Jwk),
});
/** A JSON Web Key Set. */
export type JwkSet = schema.Infer<typeof JwkSet>;

/** How a remote key set is fetched and cached. */
export interface RemoteJwksOptions {
    /** Fetch the key set. */
    readonly fetch: (request: Request) => Promise<Response>;
    /** How long a fetched set stays fresh, in milliseconds. */
    readonly maxAge: number;
    /** The shortest wait between fetches for an unknown key, in milliseconds. */
    readonly cooldown: number;
    /** How long a fetch may take, in milliseconds. */
    readonly timeout: number;
}

/** Key sets finding a token's keys, as jose's createLocalJWKSet and createRemoteJWKSet. */
export const Jwks = {
    /** Find keys in a set by the header's key identifier and algorithm, importing each key once. */
    local(set: JwkSet, crypto: Crypto): KeyLookup {
        const imported = new Map<Jwk, Promise<CryptoKey>>();

        return async (header) => {
            // refuse a header without a key of its algorithm, or with a key identifier no key has
            const found = matchKeys(set, header);
            if (found === undefined) {
                throw IdentityError.of("KEY_NOT_FOUND", "no key of the set signed the token");
            }

            // import each matched key once, and use the keys that import
            const settled = await Promise.allSettled(
                found.keys.map((jwk) => {
                    const key = imported.get(jwk) ?? Jws.importJwk(jwk, found.algorithm, crypto);
                    imported.set(jwk, key);

                    return key;
                }),
            );
            const keys = settled.flatMap((each) =>
                each.status === "fulfilled" ? [each.value] : [],
            );
            if (keys.length === 0) {
                const reasons = settled.flatMap((each): unknown[] =>
                    each.status === "rejected" ? [each.reason] : [],
                );
                throw IdentityError.of("KEY_INVALID", "no key of the set imports", {
                    cause: new AggregateError(reasons),
                });
            }

            return keys;
        };
    },

    /** Find keys in a set fetched from a URL, cached on a clock. */
    remote(
        url: URL,
        options: RemoteJwksOptions,
        environment: { readonly clock: Clock; readonly crypto: Crypto },
    ): KeyLookup {
        const remote = new RemoteJwks(url, options, environment);

        return (header) => remote.lookup(header);
    },
};

/** A fetch of a remote key set: the set, its time, and the lookup of its keys. */
interface JwksFetch {
    /** The fetched set with the lookup of its keys, each imported once. */
    readonly keys: Promise<{ readonly set: JwkSet; readonly lookup: KeyLookup }>;
    /** When the fetch began. */
    readonly at: number;
}

/** A key set fetched from a URL, used until stale and fetched again for an unknown key. */
class RemoteJwks {
    /** The set's address. */
    readonly #url: URL;
    /** How the set is fetched and cached. */
    readonly #options: RemoteJwksOptions;
    /** The clock and crypto the set is fetched and its keys imported with. */
    readonly #environment: { readonly clock: Clock; readonly crypto: Crypto };
    /** The last fetch that has not failed, absent before the first. */
    #fetched: JwksFetch | undefined;

    /** Fetch a set from a URL on demand. */
    constructor(
        url: URL,
        options: RemoteJwksOptions,
        environment: { readonly clock: Clock; readonly crypto: Crypto },
    ) {
        this.#url = url;
        this.#options = options;
        this.#environment = environment;
    }

    /** Find the keys verifying a header, fetching the set as it goes stale or lacks the key. */
    async lookup(header: JwsHeader): Promise<readonly JwsKey[]> {
        // fetch a set that is missing or stale
        const { clock } = this.#environment;
        const fetched = this.#fetched;
        const current =
            fetched === undefined || clock.now() - fetched.at >= this.#options.maxAge
                ? this.#refresh(fetched)
                : fetched;
        const { set } = await current.keys;

        // fetch once more for an unknown key after the cooldown, sharing one fetch among lookups
        const isCooled = clock.now() - current.at >= this.#options.cooldown;
        const isUnknown = matchKeys(set, header) === undefined;
        const known =
            isUnknown && isCooled
                ? this.#fetched === current
                    ? this.#refresh(current)
                    : (this.#fetched ?? current)
                : current;
        const { lookup } = await known.keys;

        return lookup(header);
    }

    /** Fetch the set after a previous fetch, falling back to the previous one once it fails. */
    #refresh(previous: JwksFetch | undefined): JwksFetch {
        // fetch the set, and use it as the last fetch
        const { clock, crypto } = this.#environment;
        const keys = fetchKeys(this.#url, this.#options, clock).then((set) => ({
            set,
            lookup: Jwks.local(set, crypto),
        }));
        const fetched = { keys, at: clock.now() };
        this.#fetched = fetched;

        // restore the previous fetch once this one fails
        keys.catch(() => {
            if (this.#fetched === fetched) {
                this.#fetched = previous;
            }
        });

        return fetched;
    }
}

/** Match a header's algorithm and key identifier in a set, absent without a fitting key. */
function matchKeys(set: JwkSet, header: JwsHeader) {
    const algorithm = JwsAlgorithm.safeParse(header.alg).data;
    const keys = set.keys.filter(
        (jwk) =>
            algorithm !== undefined &&
            Jwk.fits(jwk, algorithm) &&
            (header.kid === undefined || jwk.kid === header.kid),
    );

    return algorithm === undefined || keys.length === 0 ? undefined : { algorithm, keys };
}

/** Fetch a key set within the timeout, refusing an unavailable or invalid one. */
async function fetchKeys(url: URL, options: RemoteJwksOptions, clock: Clock): Promise<JwkSet> {
    try {
        const response = await options.fetch(
            new Request(url.href, {
                headers: { accept: "application/json" },
                signal: clock.timeout(options.timeout),
            }),
        );
        if (!response.ok) {
            throw new TypeError(`the key set answered ${response.status}`);
        }

        return JwkSet.parse(await response.json());
    } catch (error) {
        throw IdentityError.of("KEY_SET_UNAVAILABLE", `the key set at ${url.href} is unavailable`, {
            cause: error,
        });
    }
}

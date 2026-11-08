import { type Authentication, Caller, CLOCK_TOLERANCE_MILLISECONDS } from "@destack/access";
import {
    FEDERATED_TOKEN,
    KEY_CACHE_MILLISECONDS,
    KEY_TIMEOUT_MILLISECONDS,
    Nonce,
} from "@destack/access/token";
import { type DatabaseConnection, Snapshot } from "@destack/db";
import type { Environment } from "@destack/environment";
import { JwsAlgorithm, Jwks, Jwt, type KeyLookup } from "@destack/identity";
import type { PackageId } from "@destack/package";
import { Digest, schema } from "@destack/schema";
import { OAuthError } from "../authentication/token.ts";
import { trustedPublisher } from "../object/trusted-publisher.ts";

/** The path of an OIDC issuer's discovery document (OpenID Connect Discovery 1.0, 4). */
const DISCOVERY_PATH = "/.well-known/openid-configuration";

/** The fields of an issuer's discovery document the verifier reads. */
const Discovery = schema.looseObject({
    /** The issuer, which must be the one discovered. */
    issuer: schema.url(),
    /** Where the issuer publishes its signing keys. */
    jwks_uri: schema.url(),
    /** The algorithms the issuer signs its ID tokens with. */
    id_token_signing_alg_values_supported: schema.array(schema.string()),
});

/** An issuer's discovered signing keys and the algorithms it signs with. */
interface IssuerKeys {
    /** The issuer's key set, refreshed on the clock. */
    readonly keys: KeyLookup;
    /** The algorithms the issuer signs with that the verifier supports. */
    readonly algorithms: readonly JwsAlgorithm[];
}

/** Verify tokens OIDC issuers sign for the CI workflows accounts trust to publish. */
export class FederatedVerifier {
    /** The account database with the trusted publishers. */
    readonly #database: DatabaseConnection;
    /** The token endpoint, every federated token's audience. */
    readonly #endpoint: string;
    /** The environment fetching issuers' keys and verifying tokens. */
    readonly #environment: Environment;
    /** Each issuer's discovered keys, discovered once, again after a failed discovery. */
    readonly #issuers = new Map<string, Promise<IssuerKeys>>();

    /** Verify tokens for the token endpoint against the trusted publishers in a database. */
    constructor(database: DatabaseConnection, endpoint: string, environment: Environment) {
        this.#database = database;
        this.#endpoint = endpoint;
        this.#environment = environment;
    }

    /** Decide whether an OIDC issuer signed a token, as our own assertions' issuers are objects. */
    static isFederated(token: string): boolean {
        const { iss } = Jwt.claims(token);

        return iss?.startsWith("https://") === true;
    }

    /** Verify a federated token at a time, answering its trusted publisher as the caller. */
    async verify(token: string, audience: PackageId, now: number): Promise<Authentication> {
        // find the trusted publisher of the token's issuer and subject
        const { iss, sub } = Jwt.claims(token);
        const trusted =
            iss === undefined || sub === undefined
                ? undefined
                : await Snapshot.live(this.#database)
                      .read(trustedPublisher.table)
                      .findFirst({ where: { issuer: iss, subject: sub } });
        if (trusted === undefined) {
            throw OAuthError.of("INVALID_GRANT", "no account trusts this workflow");
        }

        // verify the signature by the issuer's keys, and the token's issuer, subject, audience and times
        const { keys, algorithms } = await this.#keys(trusted.issuer);
        const { claims } = await Jwt.verify(
            token,
            keys,
            {
                algorithms,
                issuer: trusted.issuer,
                subject: trusted.subject,
                audience: this.#endpoint,
                requiredClaims: ["iat", "exp", "jti"],
                clockTolerance: CLOCK_TOLERANCE_MILLISECONDS / 1000,
                now,
            },
            this.#environment.crypto,
        ).catch((error: unknown) => {
            throw OAuthError.of("INVALID_GRANT", "invalid federated token", { cause: error });
        });
        if (claims.exp === undefined || claims.jti === undefined) {
            throw OAuthError.of("INVALID_GRANT", "federated token has no expiry or identifier");
        }

        // spend the token once for its trusted publisher, until it expires
        const expiresAt = claims.exp * 1000;
        const digest = await Digest.of(
            JSON.stringify([trusted.id, claims.jti]),
            this.#environment.crypto,
        );
        await new Nonce(digest, expiresAt).spend(this.#database, now);

        return {
            caller: Caller.of(trustedPublisher.reference(trusted.scope, trusted.id)),
            credential: {
                kind: FEDERATED_TOKEN,
                id: trusted.id,
                audience,
                verifiedAt: now,
                expiresAt,
            },
        };
    }

    /** Read an issuer's keys, discovering them once per issuer. */
    #keys(issuer: string): Promise<IssuerKeys> {
        // reuse a discovery, forgetting a failed one
        const known = this.#issuers.get(issuer);
        if (known !== undefined) {
            return known;
        }
        const discovered = this.#discover(issuer);
        this.#issuers.set(issuer, discovered);
        discovered.catch(() => this.#issuers.delete(issuer));

        return discovered;
    }

    /** Discover an issuer's keys and algorithms, refusing a document of another issuer. */
    async #discover(issuer: string): Promise<IssuerKeys> {
        // read the issuer's discovery document
        const { network } = this.#environment;
        const response = await network.fetch(new Request(`${issuer}${DISCOVERY_PATH}`));
        if (!response.ok) {
            await response.body?.cancel();
            throw OAuthError.of("INVALID_GRANT", `${issuer} publishes no discovery document`);
        }
        const discovery = Discovery.parse(await response.json());
        if (discovery.issuer !== issuer) {
            throw OAuthError.of("INVALID_GRANT", `${issuer} discovers another issuer`);
        }

        // keep the algorithms the verifier supports, and the keys refreshed on the clock
        const algorithms = discovery.id_token_signing_alg_values_supported.flatMap((name) => {
            const parsed = JwsAlgorithm.safeParse(name);

            return parsed.success ? [parsed.data] : [];
        });
        const keys = Jwks.remote(
            new URL(discovery.jwks_uri),
            {
                fetch: (request) => network.fetch(request),
                maxAge: KEY_CACHE_MILLISECONDS,
                cooldown: KEY_TIMEOUT_MILLISECONDS,
                timeout: KEY_TIMEOUT_MILLISECONDS,
            },
            this.#environment,
        );

        return { keys, algorithms };
    }
}

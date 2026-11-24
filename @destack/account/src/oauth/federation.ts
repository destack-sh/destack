import {
    type Authentication,
    Caller,
    CLOCK_TOLERANCE_MILLISECONDS,
    principal,
} from "@destack/access";
import { Scope } from "@destack/sync";
import { FEDERATED_TOKEN, Nonce } from "@destack/access/token";
import type { DatabaseConnection } from "@destack/db/database";
import type { Environment } from "@destack/environment";
import { JwsAlgorithm, Jwks, Jwt, type KeyLookup } from "@destack/identity";
import type { PackageId } from "@destack/package";
import { Digest, schema } from "@destack/schema";
import { OAuthError } from "../authentication/token.ts";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import { client } from "../object/client.ts";

/** The longest interval between public key refreshes, in milliseconds. */
const KEY_CACHE_MILLISECONDS = 60000;

/** The longest key discovery wait and the pause after a failed one, in milliseconds. */
const KEY_TIMEOUT_MILLISECONDS = 5000;

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
    /** The account database with the federated clients. */
    readonly #database: DatabaseConnection;
    /** The token endpoint, every federated token's audience. */
    readonly #endpoint: string;
    /** The environment fetching issuers' keys and verifying tokens. */
    readonly #environment: Environment;
    /** Each issuer's discovered keys, discovered once, again after a failed discovery. */
    readonly #issuers = new Map<string, Promise<IssuerKeys>>();
    /** The verifier's spans and durations. */
    readonly #scope: TelemetryScope;

    /** Verify tokens for the token endpoint against the federated clients in a database. */
    constructor(database: DatabaseConnection, endpoint: string, environment: Environment) {
        // take the database, the endpoint and the environment, and trace on its telemetry
        this.#database = database;
        this.#endpoint = endpoint;
        this.#environment = environment;
        this.#scope = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
    }

    /** Decide whether an OIDC issuer signed a token, as our own assertions' issuers are objects. */
    static isFederated(token: string): boolean {
        const { iss } = Jwt.claims(token);

        return iss !== undefined && URL.canParse(iss);
    }

    /** Verify a federated token at a time, answering its client as the caller. */
    verify(token: string, audience: PackageId, now: number): Promise<Authentication> {
        return this.#scope.span("destack.federation.verify", {}, () =>
            this.#verify(token, audience, now),
        );
    }

    /** Find the token's client, verify it by its issuer's keys, and spend it once. */
    async #verify(token: string, audience: PackageId, now: number): Promise<Authentication> {
        // find the standing client of the token's issuer and subject
        const { iss: issuer, sub: subject } = Jwt.claims(token);
        const trusted =
            issuer === undefined || subject === undefined
                ? undefined
                : await this.#database
                      .snapshot()
                      .read(client.table)
                      .findFirst({ where: { issuer, subject, revokedAt: { isNull: true } } });
        if (issuer === undefined || subject === undefined || trusted === undefined) {
            throw OAuthError.of(
                "GRANT_INVALID",
                "workflow is invalid: no account trusts this workflow",
            );
        }

        // refuse a plain HTTP issuer unless the universe itself answers over plain HTTP, as a local stage does
        const isPlain = new URL(issuer).protocol !== "https:";
        if (isPlain && new URL(this.#endpoint).protocol === "https:") {
            throw OAuthError.of(
                "GRANT_INVALID",
                `issuer '${issuer}' is invalid: a universe over HTTPS trusts HTTPS issuers alone`,
            );
        }

        // verify the signature by the issuer's keys, and the token's issuer, subject, audience and times
        const { keys, algorithms } = await this.#keys(issuer);
        const { claims } = await Jwt.verify(
            token,
            keys,
            {
                algorithms,
                issuer,
                subject,
                audience: this.#endpoint,
                requiredClaims: ["iat", "exp", "jti"],
                clockTolerance: CLOCK_TOLERANCE_MILLISECONDS / 1000,
                now,
            },
            this.#environment.crypto,
        ).catch((error: unknown) => {
            throw OAuthError.of("GRANT_INVALID", "federated token is invalid: it does not verify", {
                cause: error,
            });
        });
        if (claims.exp === undefined || claims.jti === undefined) {
            throw OAuthError.of(
                "GRANT_INVALID",
                "federated token is invalid: it has no expiry or identifier",
            );
        }

        // spend the token once for its client, until it expires
        const expiresAt = claims.exp * 1000;
        const digest = await Digest.of(
            JSON.stringify([trusted.id, claims.jti]),
            this.#environment.crypto,
        );
        await new Nonce(digest, expiresAt).spend(this.#database, now);

        return {
            caller: Caller.of(principal.client.reference(Scope.universe.id, trusted.id)),
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
            throw OAuthError.of(
                "GRANT_INVALID",
                `issuer '${issuer}' is invalid: it publishes no discovery document`,
            );
        }
        const discovery = Discovery.parse(await response.json());
        if (discovery.issuer !== issuer) {
            throw OAuthError.of(
                "GRANT_INVALID",
                `issuer '${issuer}' is invalid: it discovers another issuer`,
            );
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

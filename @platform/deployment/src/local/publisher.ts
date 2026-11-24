import type { Signer } from "@destack/account/client";
import type { Environment } from "@destack/environment";
import { Jwt, KeyPair, PublicKey } from "@destack/identity";
import { Uuid } from "@destack/schema";

/** The local release client's signing key, which only this computer uses, as the root keys. */
const PUBLISHER_KEY =
    '{"kty":"EC","crv":"P-256","x":"ToWboKl5usLrd2cYxnqzwwkpXz0Hq9JaF6QlcxzLEeA","y":"pa20eCzQEnH7tsaZKhILn6HABx-J-KHxoZF99O9y8Kc","d":"mz6FnqX7wVAORwtOk52uGhTZAVgqlsD0nAXSER5YyzQ"}';

/** The path of an OIDC issuer's discovery document (OpenID Connect Discovery 1.0, 4). */
const DISCOVERY_PATH = "/.well-known/openid-configuration";

/** The path the issuer publishes its signing keys at. */
const KEYS_PATH = "/keys";

/** The subject the local client signs as, as the release workflow signs as its environment. */
const SUBJECT = "local";

/** How long an assertion stands, in seconds: one token request, with room for clock skew. */
const ASSERTION_SECONDS = 300;

/** A local universe's release workflow: the OIDC issuer of the one client publishing its build. */
export class LocalPublisher {
    /** The issuer's origin. */
    readonly issuer: string;
    /** The key signing the client's assertions. */
    readonly #keys: KeyPair;
    /** The environment signing assertions on its clock. */
    readonly #environment: Environment;

    /** Issue a client's assertions at an origin with a key. */
    constructor(issuer: string, keys: KeyPair, environment: Environment) {
        // take the issuer, its key and the environment
        this.issuer = issuer;
        this.#keys = keys;
        this.#environment = environment;
    }

    /** Open the issuer at an origin with the local key. */
    static async open(issuer: string, environment: Environment): Promise<LocalPublisher> {
        const keys = await KeyPair.read(PUBLISHER_KEY, environment.crypto);

        return new LocalPublisher(issuer, keys, environment);
    }

    /** The client a universe trusts to publish the platform's releases. */
    get client(): { readonly name: string; readonly issuer: string; readonly subject: string } {
        return { name: "Local", issuer: this.issuer, subject: SUBJECT };
    }

    /** The signer of the client's assertions. */
    get signer(): Signer {
        return { assert: (endpoint) => this.assert(endpoint) };
    }

    /** Sign an assertion of the client for a token endpoint now. */
    async assert(endpoint: string): Promise<string> {
        // claim the client for the endpoint, once, for a short while
        const { crypto, clock } = this.#environment;
        const now = Math.floor(clock.now() / 1000);
        const claims = {
            iss: this.issuer,
            sub: SUBJECT,
            aud: endpoint,
            iat: now,
            exp: now + ASSERTION_SECONDS,
            jti: Uuid.v7(this.#environment),
        };

        // sign it with the key its key set names
        const kid = await PublicKey.thumbprint(this.#keys.publicKey, crypto);

        return Jwt.sign(claims, { typ: "JWT", alg: "ES256", kid }, this.#keys.privateKey, crypto);
    }

    /** Answer the issuer's discovery document and signing keys. */
    async fetch(request: Request): Promise<Response> {
        // describe the issuer and where its keys are
        const { pathname } = new URL(request.url);
        if (pathname === DISCOVERY_PATH) {
            return Response.json({
                issuer: this.issuer,
                jwks_uri: `${this.issuer}${KEYS_PATH}`,
                id_token_signing_alg_values_supported: ["ES256"],
            });
        }
        // publish its key
        else if (pathname === KEYS_PATH) {
            return Response.json(
                await PublicKey.keySet([this.#keys.publicKey], this.#environment.crypto),
            );
        }
        // answer nothing else
        else {
            return new Response(null, { status: 404 });
        }
    }
}

import type { InstalledServerContext } from "@destack/object/workload";
import type { JsonValue } from "@destack/schema";
import type { Crypto } from "@destack/environment";
import {
    IdentityError,
    Jwk,
    Jwks,
    Jws,
    Jwt,
    type JwsAlgorithm,
    type JwtClaims,
    type KeyLookup,
} from "@destack/identity";
import { Bearer, KEY_CACHE_MILLISECONDS, KEY_TIMEOUT_MILLISECONDS } from "@destack/access/token";
import { ObjectError, RequestBody } from "@destack/object";
import type { HttpsBinding } from "@destack/object";
import {
    WebhookDelivery,
    type WebhookMessage,
    type WebhookParameters,
    WebhookSignature,
    type WebhookVerifier,
} from "./webhook.ts";
import type {} from "@destack/package/import-meta";

/** How long a signed bearer token lasts, five minutes as the Standard Webhooks tolerance. */
const TOKEN_SECONDS = 300;

/** The signing algorithm of the bearer tokens this scheme signs (RFC 7518 3.4). */
const ALGORITHM: JwsAlgorithm = "ES256";

/** The algorithms of the sender's public keys a token may verify with (RFC 8725 3.1). */
const VERIFY_ALGORITHMS = ["ES256", "ES384", "EdDSA", "Ed25519", "RS256"] as const;

/** What a JWT bearer webhook scheme checks. */
export interface BearerOptions {
    /** The issuers the token may have, the first signing (RFC 7519 4.1.1). */
    readonly issuer: string | readonly [string, ...string[]];
    /** The audience the token must have, the receiving endpoint (RFC 7519 4.1.3). */
    readonly audience: string;
    /** The sender's key set (RFC 7517 5): an https resource and the path below it. */
    readonly keys: { readonly resource: HttpsBinding; readonly path: string };
    /** Read the delivery's identifier and event from the body and the verified claims. */
    readonly delivery: (
        payload: JsonValue,
        claims: JwtClaims,
    ) => { readonly id: string; readonly event: string };
}

/** A webhook scheme whose sender signs a JWT bearer token for the endpoint (RFC 7519, 6750 2.1). */
export class BearerSignature implements WebhookSignature {
    /** The scheme's name. */
    readonly name: string;
    /** Whether the scheme verifies with a shared secret, which it does not. */
    readonly isKeyed: boolean;
    /** The checked issuer, audience, keys and delivery. */
    readonly options: BearerOptions;

    /** Check tokens of an issuer for an audience against its published keys. */
    constructor(options: BearerOptions) {
        this.name = "bearer";
        this.isKeyed = false;
        this.options = options;
    }

    /** Sign a message as the issuer with a private JWK, as a sender's bearer token. */
    async sign(message: WebhookMessage, secret: string, crypto: Crypto): Promise<Headers> {
        // read the private key and the first issuer
        const jwk = Jwk.parse(JSON.parse(secret));
        const key = await Jws.importJwk(jwk, ALGORITHM, crypto);
        const issuer =
            typeof this.options.issuer === "string" ? this.options.issuer : this.options.issuer[0];

        // sign a short-lived token with the message for the audience
        const seconds = Math.floor(message.sentAt / 1000);
        const claims = {
            jti: message.id,
            iss: issuer,
            aud: this.options.audience,
            iat: seconds,
            exp: seconds + TOKEN_SECONDS,
        };
        const header = { alg: ALGORITHM, ...(jwk.kid === undefined ? {} : { kid: jwk.kid }) };
        const token = await Jwt.sign(claims, header, key, crypto);

        return new Headers({ authorization: `Bearer ${token}` });
    }

    /** Build an installation's verifier, reading the sender's keys through its https resource. */
    verifier(
        context: Pick<InstalledServerContext, "resources" | "keys" | "environment">,
    ): WebhookVerifier {
        // fetch the remote key set below the bound address, cached for the verifier's deliveries
        const { environment } = context;
        const https = this.options.keys.resource.get(context.resources);
        const { path } = this.options.keys;
        const keys = Jwks.remote(
            new URL(`${https.url}${path}`),
            {
                fetch: (request) => https.fetch(path, request),
                maxAge: KEY_CACHE_MILLISECONDS,
                cooldown: KEY_TIMEOUT_MILLISECONDS,
                timeout: KEY_TIMEOUT_MILLISECONDS,
            },
            environment,
        );

        return (request, _secret, parameters, now) =>
            this.#verify(keys, request, parameters, now, environment.crypto);
    }

    /** Verify the bearer token's signature, issuer, audience and lifetime. */
    async #verify(
        keys: KeyLookup,
        request: Request,
        parameters: WebhookParameters,
        now: number,
        crypto: Crypto,
    ): Promise<WebhookDelivery> {
        // require a bearer token
        const token = Bearer.token(request.headers.get("authorization"));
        if (token === undefined) {
            throw ObjectError.of("SIGNATURE_MISSING", "webhook has no bearer token");
        }

        // verify it against the sender's keys
        let claims: JwtClaims;
        try {
            const options = {
                algorithms: VERIFY_ALGORITHMS,
                now,
                issuer: [this.options.issuer].flat(),
                audience: this.options.audience,
            };
            const verified = await Jwt.verify(token, keys, options, crypto);
            claims = verified.claims;
        } catch (error) {
            // rethrow a failure of the sender's keys rather than the token
            const isKeyFailure =
                IdentityError.is(error, "KEY_SET_UNAVAILABLE") ||
                IdentityError.is(error, "KEY_INVALID");
            if (!IdentityError.is(error) || isKeyFailure) {
                throw error;
            }
            throw BearerSignature.#refusal(error);
        }

        // read the delivery the body identifies
        const payload = WebhookSignature.payload(
            await RequestBody.text(request, "a webhook delivery"),
        );
        const { id, event } = this.options.delivery(payload, claims);

        return WebhookDelivery.parse({ id, event, payload, parameters, receivedAt: now });
    }

    /** Refuse a bearer token whose signature fails as a mismatch, else as identity refused it. */
    static #refusal(error: Error): Error {
        // refuse a token whose signature no key of the sender verifies
        if (
            IdentityError.is(error, "SIGNATURE_INVALID") ||
            IdentityError.is(error, "KEY_NOT_FOUND")
        ) {
            return ObjectError.of(
                "SIGNATURE_MISMATCH",
                "webhook bearer token signature does not match",
            );
        }

        return error;
    }
}

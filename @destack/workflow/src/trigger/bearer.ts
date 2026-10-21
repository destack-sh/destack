import type { ServerContext } from "@destack/object/workload";
import { schema, type JsonValue } from "@destack/schema";
import {
    createRemoteJWKSet,
    customFetch,
    errors,
    importJWK,
    jwtVerify,
    SignJWT,
    type JWTPayload,
    type JWTVerifyGetKey,
} from "jose";
import { Bearer, KEY_CACHE_MILLISECONDS, KEY_TIMEOUT_MILLISECONDS } from "@destack/access/token";
import { RequestBody, CallError } from "@destack/object";
import type { HttpsBinding } from "@destack/object";
import {
    WebhookDelivery,
    type WebhookMessage,
    type WebhookParameters,
    WebhookSignature,
    type WebhookVerifier,
} from "./webhook.ts";

/** How long a signed bearer token lasts, five minutes as the Standard Webhooks tolerance. */
const TOKEN_SECONDS = 300;

/** The signing algorithm of the bearer tokens this scheme signs (RFC 7518 3.4). */
const ALGORITHM = "ES256";

/** A private signing key in JWK form (RFC 7517), as a bearer scheme's signing secret stores it. */
const SigningKey = schema.looseObject({
    kty: schema.string(),
    kid: schema.string().exactOptional(),
});

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
        claims: JWTPayload,
    ) => { readonly id: string; readonly event: string };
}

/** A webhook scheme whose sender signs a JWT bearer token for the endpoint (RFC 7519, 6750 2.1). */
export class BearerSignature implements WebhookSignature {
    /** The scheme's name. */
    readonly name = "bearer";
    /** The checked issuer, audience, keys and delivery. */
    readonly options: BearerOptions;

    /** Check tokens of an issuer for an audience against its published keys. */
    constructor(options: BearerOptions) {
        this.options = options;
    }

    /** Sign a message as the issuer with a private JWK, as a sender's bearer token. */
    async sign(message: WebhookMessage, secret: string): Promise<Headers> {
        // sign a short-lived token with the message for the audience
        const jwk = SigningKey.parse(JSON.parse(secret));
        const key = await importJWK(jwk, ALGORITHM);
        const issuer =
            typeof this.options.issuer === "string" ? this.options.issuer : this.options.issuer[0];
        const seconds = Math.floor(message.sentAt / 1000);
        const token = await new SignJWT({ jti: message.id })
            .setProtectedHeader({
                alg: ALGORITHM,
                ...(jwk.kid === undefined ? {} : { kid: jwk.kid }),
            })
            .setIssuer(issuer)
            .setAudience(this.options.audience)
            .setIssuedAt(seconds)
            .setExpirationTime(seconds + TOKEN_SECONDS)
            .sign(key);

        return new Headers({ authorization: `Bearer ${token}` });
    }

    /** Build an installation's verifier, reading the sender's keys through its https resource. */
    verifier(context: Pick<ServerContext, "resources" | "keys">): WebhookVerifier {
        // fetch the remote key set below the bound address, cached for the verifier's deliveries
        const https = this.options.keys.resource.get(context.resources);
        const { path } = this.options.keys;
        const keys = createRemoteJWKSet(new URL(`${https.url}${path}`), {
            cacheMaxAge: KEY_CACHE_MILLISECONDS,
            cooldownDuration: KEY_TIMEOUT_MILLISECONDS,
            timeoutDuration: KEY_TIMEOUT_MILLISECONDS,
            [customFetch]: (_url, request) => https.fetch(path, request),
        });

        return (request, _secret, parameters, now) => this.#verify(keys, request, parameters, now);
    }

    /** Verify the bearer token's signature, issuer, audience and lifetime. */
    async #verify(
        keys: JWTVerifyGetKey,
        request: Request,
        parameters: WebhookParameters,
        now: number,
    ): Promise<WebhookDelivery> {
        // require a bearer token
        const token = Bearer.token(request.headers.get("authorization"));
        if (token === undefined) {
            throw new CallError("UNAUTHORIZED", { message: "webhook has no bearer token" });
        }

        // verify it against the sender's keys
        let claims: JWTPayload;
        try {
            const verified = await jwtVerify(token, keys, {
                issuer: [this.options.issuer].flat(),
                audience: this.options.audience,
                currentDate: new Date(now),
            });
            claims = verified.payload;
        } catch (error) {
            if (!(error instanceof errors.JOSEError)) {
                throw error;
            }
            throw new CallError("UNAUTHORIZED", {
                message: "webhook bearer token does not belong to this endpoint's issuer",
            });
        }

        // read the delivery the body identifies
        const payload = WebhookSignature.payload(
            await RequestBody.text(request, "a webhook delivery"),
        );
        const { id, event } = this.options.delivery(payload, claims);

        return WebhookDelivery.parse({ id, event, payload, parameters, receivedAt: now });
    }
}

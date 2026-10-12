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
import { Bearer } from "../authentication/bearer.ts";
import { KEY_CACHE_MILLISECONDS, KEY_TIMEOUT_MILLISECONDS } from "../authentication/token.ts";
import { RequestBody } from "../request/body.ts";
import type { HttpsBinding, HttpsClient } from "../declare/https.ts";
import { ServiceError } from "../error/index.ts";
import {
    WebhookDelivery,
    type WebhookContext,
    type WebhookMessage,
    type WebhookParameters,
    WebhookSignature,
} from "./webhook.ts";

/** How long a signed bearer token lasts, five minutes as the Standard Webhooks tolerance. */
const TOKEN_SECONDS = 300;

/** The signing algorithm of the bearer tokens this scheme signs (RFC 7518 3.4). */
const ALGORITHM = "ES256";

/** A private signing key in JWK form (RFC 7517), as a bearer scheme's signing secret holds it. */
const SigningKey = schema.looseObject({
    kty: schema.string(),
    kid: schema.string().exactOptional(),
});

/** What a JWT bearer webhook scheme checks. */
export interface BearerOptions {
    /** The issuers the token may carry, the first signing (RFC 7519 4.1.1). */
    readonly issuer: string | readonly [string, ...string[]];
    /** The audience the token must carry, the receiving endpoint (RFC 7519 4.1.3). */
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
    /** The cached key sets, by the https client fetching them. */
    readonly #keys = new WeakMap<HttpsClient, JWTVerifyGetKey>();

    /** Check tokens of an issuer for an audience against its published keys. */
    constructor(options: BearerOptions) {
        this.options = options;
    }

    /** Sign a message as the issuer with a private JWK, as a sender's bearer token. */
    async sign(message: WebhookMessage, secret: string): Promise<Headers> {
        // sign a short-lived token carrying the message for the audience
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

    /** Verify the bearer token's signature, issuer, audience and lifetime. */
    async verify(
        request: Request,
        _secret: string | undefined,
        parameters: WebhookParameters,
        now: number,
        context: WebhookContext,
    ): Promise<WebhookDelivery> {
        // require a bearer token
        const token = Bearer.token(request.headers.get("authorization"));
        if (token === undefined) {
            throw new ServiceError("UNAUTHORIZED", { message: "webhook carries no bearer token" });
        }

        // verify it against the sender's keys
        let claims: JWTPayload;
        try {
            const verified = await jwtVerify(token, this.#keySet(context), {
                issuer: [this.options.issuer].flat(),
                audience: this.options.audience,
                currentDate: new Date(now),
            });
            claims = verified.payload;
        } catch (error) {
            if (!(error instanceof errors.JOSEError)) {
                throw error;
            }
            throw new ServiceError("UNAUTHORIZED", {
                message: "webhook bearer token is not the issuer's for this endpoint",
            });
        }

        // read the delivery the body identifies
        const payload = WebhookSignature.payload(
            await RequestBody.text(request, "a webhook delivery"),
        );
        const { id, event } = this.options.delivery(payload, claims);

        return WebhookDelivery.parse({ id, event, payload, parameters, receivedAt: now });
    }

    /** Read the sender's key set through the installation's https resource. */
    #keySet(context: WebhookContext): JWTVerifyGetKey {
        // keep one remote set per client, fetched below its bound address
        const https = this.options.keys.resource.get(context.resources);
        const kept = this.#keys.get(https);
        if (kept !== undefined) {
            return kept;
        }

        // read through the client, which carries the instance's credential
        const { path } = this.options.keys;
        const keys = createRemoteJWKSet(new URL(`${https.url}${path}`), {
            cacheMaxAge: KEY_CACHE_MILLISECONDS,
            cooldownDuration: KEY_TIMEOUT_MILLISECONDS,
            timeoutDuration: KEY_TIMEOUT_MILLISECONDS,
            [customFetch]: (_url, request) => https.fetch(path, request),
        });
        this.#keys.set(https, keys);

        return keys;
    }
}

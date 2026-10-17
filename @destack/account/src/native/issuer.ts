import { type Fetch, PackageMount } from "@destack/runtime";
import { ListPage } from "@destack/object";
import type { StepUp } from "@destack/access";
import { PackageId } from "@destack/package";
import { type Identifier, schema } from "@destack/schema";
import { createClient } from "@destack/runtime/client";
import { CallError } from "@destack/runtime/error";
import { Pkce } from "@destack/runtime/authentication";
import { DEVICE_CODE_GRANT_TYPE } from "@better-auth/oauth-provider";
import { TOKEN_PATH } from "../authentication/path.ts";
import { TokenResponse } from "../authentication/token.ts";
import { EMAIL_CODE } from "../oauth/code.ts";
import { ACCESS_TOKEN_TYPE, TOKEN_EXCHANGE } from "../oauth/exchange.ts";
import accountPackage from "../package.ts";
import type { Residency } from "../object/residency.ts";
import type { SignUp } from "../better-auth/sign-up.ts";

/** The milliseconds in a second, as OAuth counts expiries and ages in seconds. */
const SECOND_MILLISECONDS = 1000;

/** How long a browser sign-in may take, in milliseconds: ten minutes for the person to sign in. */
const BROWSER_MILLISECONDS = 10 * 60_000;

/** The scopes a native sign-in asks for: the person's profile, and a refresh token (OIDC). */
const SIGN_IN_SCOPES = "openid profile offline_access";

/** A refused token request (RFC 6749 5.2, RFC 8628 3.5). */
const ErrorResponse = schema.looseObject({
    /** The error code. */
    error: schema.string().min(1),
    /** What the error means, for the person (RFC 6749 5.2). */
    error_description: schema.string().exactOptional(),
});

/** The issuer's answer to mailing a sign-in code: Better Auth's email OTP's. */
const CodeSent = schema.looseObject({
    /** Whether the code was mailed. */
    success: schema.literal(true),
});

/** A device authorization (RFC 8628 3.2). */
const DeviceResponse = schema.looseObject({
    /** The code the native device polls with. */
    device_code: schema.string().min(1),
    /** The code the person enters. */
    user_code: schema.string().min(1),
    /** The page the person opens. */
    verification_uri: schema.url(),
    /** The page with the user code filled in. */
    verification_uri_complete: schema.url().exactOptional(),
    /** The lifetime of the codes, in seconds. */
    expires_in: schema.number().int().positive(),
});

/** The claims of an ID token the native device reads: the signed-in person (OIDC Core 2). */
const IdClaims = schema.looseObject({
    /** The person's user. */
    sub: schema.identifier("user"),
});

/** A started browser sign-in: the page the person opens, and the PKCE verifier. */
export interface BrowserAuthorization {
    /** The page the person opens. */
    readonly url: string;
    /** The PKCE code verifier. */
    readonly verifier: string;
    /** The end of the sign-in, in UTC epoch milliseconds. */
    readonly expiresAt: number;
}

/** A started device sign-in: the page and code the person enters, and the device code. */
export interface DeviceAuthorization {
    /** The page the person opens. */
    readonly url: string;
    /** The code the person enters. */
    readonly userCode: string;
    /** The device code the client polls with. */
    readonly deviceCode: string;
    /** The end of the sign-in, in UTC epoch milliseconds. */
    readonly expiresAt: number;
}

/** The tokens a sign-in or refresh grants. */
export interface Tokens {
    /** The access token for the account package. */
    readonly accessToken: string;
    /** When the access token expires, in UTC epoch milliseconds. */
    readonly expiresAt: number;
    /** The refresh token of the signed-in person. */
    readonly refreshToken: string;
}

/** A completed sign-in: the client's tokens and the person of its ID token. */
export interface SignedIn {
    /** The client's tokens. */
    readonly tokens: Tokens;
    /** The signed-in person's user. */
    readonly userId: Identifier<"user">;
}

/** The outcome of polling a device code. */
export type DevicePoll =
    | { readonly status: "granted"; readonly signedIn: SignedIn }
    | { readonly status: "pending" | "slow" | "refused" };

/** What a person's package token acts in: a space, a grant to a delegate, both, or the universe. */
export interface TokenBinding {
    /** The space the token acts in, absent toward a universe package. */
    readonly spaceId?: Identifier<"space">;
    /** The grant to a delegate the token acts under. */
    readonly grant?: Identifier<"grant">;
}

/** The issuer a native device signs people in with, by redirect or device code. */
export class Issuer {
    /** The issuer's URL, the account package's oauth adapter. */
    readonly url: string;
    /** The fetch calling the issuer. */
    readonly #fetch: Fetch;

    /** Call an issuer through a fetch. */
    constructor(url: string, fetch: Fetch = globalThis.fetch) {
        this.url = url.replace(/\/$/u, "");
        this.#fetch = fetch;
    }

    /** Start a browser sign-in of a client that returns to a loopback redirect with a state. */
    async authorize(
        clientId: Identifier<"client">,
        redirect: string,
        state: string,
        now: number,
        stepUp?: StepUp,
    ): Promise<BrowserAuthorization> {
        // prove the flow with a verifier its S256 challenge commits to
        const verifier = Pkce.verifier();
        const url = new URL(`${this.url}/oauth2/authorize`);
        url.search = new URLSearchParams({
            response_type: "code",
            client_id: clientId,
            redirect_uri: redirect,
            state,
            scope: SIGN_IN_SCOPES,
            resource: PackageId.resource(accountPackage.id),
            code_challenge: await Pkce.challenge(verifier),
            code_challenge_method: "S256",
            ...(stepUp?.maxAge === undefined
                ? {}
                : {
                      prompt: "login",
                      max_age: String(Math.floor(stepUp.maxAge / SECOND_MILLISECONDS)),
                  }),
        }).toString();

        return { url: url.href, verifier, expiresAt: now + BROWSER_MILLISECONDS };
    }

    /** Redeem a redirect's authorization code for the client's tokens. */
    async redeem(
        clientId: Identifier<"client">,
        code: string,
        verifier: string,
        redirect: string,
    ): Promise<SignedIn> {
        // post the code with its verifier
        const response = await this.#post(TOKEN_PATH, {
            grant_type: "authorization_code",
            code,
            redirect_uri: redirect,
            client_id: clientId,
            code_verifier: verifier,
        });
        if (!response.ok) {
            const { error, error_description: description } = await Issuer.#read(
                response,
                ErrorResponse,
            );
            throw new CallError("UNAUTHORIZED", {
                message: `the issuer refused the code: ${description ?? error}`,
            });
        }

        return Issuer.#signedIn(await Issuer.#read(response, TokenResponse));
    }

    /** Mail a person a sign-in code. */
    async sendCode(email: string): Promise<void> {
        // post the email
        const response = await this.#fetch(
            new Request(`${this.url}/email-otp/send-verification-otp`, {
                method: "POST",
                headers: { "content-type": "application/json" },
                body: JSON.stringify({ email, type: "sign-in" }),
            }),
        );
        if (!response.ok) {
            throw new CallError("BAD_GATEWAY", {
                message: `the issuer refused to mail a code with status ${response.status}`,
            });
        }

        // read the answer to its end
        await Issuer.#read(response, CodeSent);
    }

    /** Redeem a mailed code for a new client's tokens, signing a new person up with a sign-up. */
    async redeemCode(
        clientId: Identifier<"client">,
        email: string,
        code: string,
        signUp?: schema.Input<typeof SignUp>,
    ): Promise<SignedIn> {
        // post the code for the account package's resource, with a new person's sign-up
        const response = await this.#post(TOKEN_PATH, {
            grant_type: EMAIL_CODE,
            email,
            code,
            client_id: clientId,
            resource: PackageId.resource(accountPackage.id),
            ...(signUp === undefined
                ? {}
                : {
                      name: signUp.name,
                      handle: signUp.handle,
                      residency_id: signUp.residencyId,
                      time_zone: signUp.timeZone,
                      locale: signUp.locale,
                  }),
        });
        if (!response.ok) {
            const { error, error_description: description } = await Issuer.#read(
                response,
                ErrorResponse,
            );
            throw new CallError("UNAUTHORIZED", {
                message: `the issuer refused the code: ${description ?? error}`,
            });
        }

        return Issuer.#signedIn(await Issuer.#read(response, TokenResponse));
    }

    /** Start a device sign-in of a client. */
    async device(clientId: Identifier<"client">, now: number): Promise<DeviceAuthorization> {
        // ask the issuer for the codes
        const response = await this.#post("/device/code", {
            client_id: clientId,
            scope: SIGN_IN_SCOPES,
            resource: PackageId.resource(accountPackage.id),
        });
        if (!response.ok) {
            throw new CallError("BAD_GATEWAY", {
                message: `the issuer refused a device code with status ${response.status}`,
            });
        }

        // show the page with the code filled in and poll with the device code
        const device = await Issuer.#read(response, DeviceResponse);

        return {
            url: device.verification_uri_complete ?? device.verification_uri,
            userCode: device.user_code,
            deviceCode: device.device_code,
            expiresAt: now + device.expires_in * SECOND_MILLISECONDS,
        };
    }

    /** Poll a device code once. */
    async poll(clientId: Identifier<"client">, deviceCode: string): Promise<DevicePoll> {
        // take the granted tokens
        const response = await this.#post(TOKEN_PATH, {
            grant_type: DEVICE_CODE_GRANT_TYPE,
            device_code: deviceCode,
            client_id: clientId,
        });
        if (response.ok) {
            return {
                status: "granted",
                signedIn: Issuer.#signedIn(await Issuer.#read(response, TokenResponse)),
            };
        }

        // wait while the person decides
        const { error, error_description: description } = await Issuer.#read(
            response,
            ErrorResponse,
        );
        if (error === "authorization_pending") {
            return { status: "pending" };
        }
        // wait longer when asked to slow down
        else if (error === "slow_down") {
            return { status: "slow" };
        }
        // give up once the person refused or the code expired
        else if (error === "access_denied" || error === "expired_token") {
            return { status: "refused" };
        }
        // fail loudly on any other error
        else {
            throw new CallError("BAD_GATEWAY", {
                message: `the issuer refused the device code: ${description ?? error}`,
            });
        }
    }

    /** Refresh a client's tokens with its refresh token. */
    refresh(clientId: Identifier<"client">, refreshToken: string): Promise<Tokens> {
        return this.#tokens(
            { grant_type: "refresh_token", refresh_token: refreshToken, client_id: clientId },
            "the issuer refused the refresh token",
        );
    }

    /** Exchange an access token for one of a package, in a space or under a grant (RFC 8693). */
    async exchange(
        accessToken: string,
        audience: PackageId,
        bound: TokenBinding,
    ): Promise<{ readonly accessToken: string; readonly expiresAt: number }> {
        // post the person's access token for the package's resource
        const response = await this.#post(TOKEN_PATH, {
            grant_type: TOKEN_EXCHANGE,
            subject_token: accessToken,
            subject_token_type: ACCESS_TOKEN_TYPE,
            resource: PackageId.resource(audience),
            ...(bound.spaceId === undefined ? {} : { space: bound.spaceId }),
            ...(bound.grant === undefined ? {} : { grant: bound.grant }),
        });
        if (!response.ok) {
            const { error, error_description: description } = await Issuer.#read(
                response,
                ErrorResponse,
            );
            throw new CallError("UNAUTHORIZED", {
                message: `the issuer refused the exchange: ${description ?? error}`,
            });
        }

        return TokenResponse.granted(await Issuer.#read(response, TokenResponse));
    }

    /** Revoke a client's refresh token at the issuer (RFC 7009). */
    async revoke(clientId: Identifier<"client">, refreshToken: string): Promise<void> {
        // post the refresh token
        const response = await this.#post("/oauth2/revoke", {
            token: refreshToken,
            token_type_hint: "refresh_token",
            client_id: clientId,
        });
        if (!response.ok) {
            throw new CallError("BAD_GATEWAY", {
                message: `the issuer refused the revocation with status ${response.status}`,
            });
        }

        // read the answer to its end, which RFC 7009 leaves without content
        await response.arrayBuffer();
    }

    /** List the residencies a new person's account may store data in. */
    async residencies(): Promise<Pick<Residency, "id" | "name">[]> {
        const items = await Array.fromAsync(ListPage.all(this.accounts().residency.list, {}));

        return items.map(({ id, name }) => ({ id, name }));
    }

    /** Connect to the account package at the issuer, with an access token when given. */
    accounts(accessToken?: string) {
        return createClient(accountPackage, {
            url: PackageMount.url(new URL(this.url).origin, accountPackage.id),
            ...(accessToken === undefined
                ? {}
                : { headers: { authorization: `Bearer ${accessToken}` } }),
            fetch: this.#fetch,
        });
    }

    /** Post a token request, refusing a refused one with a reason. */
    async #tokens(form: Readonly<Record<string, string>>, refusal: string): Promise<Tokens> {
        const response = await this.#post(TOKEN_PATH, form);
        if (!response.ok) {
            const { error, error_description: description } = await Issuer.#read(
                response,
                ErrorResponse,
            );
            throw new CallError("UNAUTHORIZED", { message: `${refusal}: ${description ?? error}` });
        }

        return Issuer.#granted(await Issuer.#read(response, TokenResponse));
    }

    /** Read granted tokens, refusing a grant without a refresh token. */
    static #granted(tokens: TokenResponse): Tokens {
        if (tokens.refresh_token === undefined) {
            throw new CallError("BAD_GATEWAY", { message: "the issuer granted no refresh token" });
        }

        return { ...TokenResponse.granted(tokens), refreshToken: tokens.refresh_token };
    }

    /** Read a sign-in's tokens and the person of its ID token. */
    static #signedIn(granted: TokenResponse): SignedIn {
        // refuse a sign-in without an ID token
        if (granted.id_token === undefined) {
            throw new CallError("BAD_GATEWAY", { message: "the issuer granted no ID token" });
        }

        // refuse an ID token without a payload
        const [, payload] = granted.id_token.split(".");
        if (payload === undefined) {
            throw new CallError("BAD_GATEWAY", {
                message: "the issuer granted a malformed ID token",
            });
        }

        // read the person from its claims, trusted as the token endpoint's own answer (OIDC Core 3.1.3.7)
        const claims = new TextDecoder().decode(
            Uint8Array.fromBase64(payload, { alphabet: "base64url" }),
        );
        const { sub } = IdClaims.parse(JSON.parse(claims));

        return { tokens: Issuer.#granted(granted), userId: sub };
    }

    /** Post a form to one of the issuer's endpoints. */
    #post(path: string, form: Readonly<Record<string, string>>): Promise<Response> {
        return this.#fetch(
            new Request(`${this.url}${path}`, {
                method: "POST",
                headers: { "content-type": "application/x-www-form-urlencoded" },
                body: new URLSearchParams(form),
            }),
        );
    }

    /** Read an issuer's JSON answer, failing as a bad gateway on a malformed answer. */
    static async #read<Definition extends schema.Schema>(
        response: Response,
        definition: Definition,
    ): Promise<schema.Infer<Definition>> {
        // parse the body as JSON
        const body = await response.text();
        let value: unknown;
        try {
            value = JSON.parse(body);
        } catch (error) {
            if (!(error instanceof SyntaxError)) {
                throw error;
            }
        }

        // answer a bad gateway for a value outside the schema
        const parsed = definition.safeParse(value);
        if (!parsed.success) {
            throw new CallError("BAD_GATEWAY", {
                message: `the issuer answered ${response.status} with an unreadable body`,
            });
        }

        return parsed.data;
    }
}

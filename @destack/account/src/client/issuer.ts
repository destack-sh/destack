import type { Environment } from "@destack/environment";
import { GRANT_DETAIL, type GrantRequest } from "../authentication/request.ts";
import { type Fetch, PackageMount } from "@destack/object";
import { ListPage } from "@destack/object";
import type { StepUp } from "@destack/access";
import { PackageId } from "@destack/package";
import { type Identifier, schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { Bookmark, BOOKMARK_HEADER, createClient } from "@destack/object/client";
import { type PartitionId, Pkce } from "@destack/access/token";
import {
    AUTHORIZATION_PATH,
    DEVICE_AUTHORIZATION_PATH,
    REVOCATION_PATH,
    TOKEN_PATH,
} from "../authentication/path.ts";
import { ErrorResponse, TokenResponse, SECOND_MILLISECONDS } from "../authentication/token.ts";
import { DEVICE_CODE, EMAIL_CODE } from "../oauth/party.ts";
import { type SubjectToken, TOKEN_EXCHANGE } from "../oauth/exchange.ts";
import accountPackage from "../package.ts";
import type { Residency } from "../object/residency.ts";
import type { SignUp } from "../better-auth/sign-up.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

/** The default wait between device polls, in seconds (RFC 8628 3.2). */
const DEVICE_INTERVAL_SECONDS = 5;

/** How long a browser sign-in may take, in milliseconds: ten minutes for the person to sign in. */
const BROWSER_MILLISECONDS = 10 * 60_000;

/** The scopes a device signs in with: the person's profile, and a refresh token (OIDC). */
export const DEVICE_SCOPES = ["openid", "profile", "offline_access"] as const;

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
    /** How long the device waits between polls, in seconds. */
    interval: schema.number().int().positive().exactOptional(),
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
    /** How long the client waits between polls, in milliseconds. */
    readonly interval: number;
    /** The end of the sign-in, in UTC epoch milliseconds. */
    readonly expiresAt: number;
}

/** The tokens a sign-in or refresh grants. */
export interface TokenSet {
    /** The access token for the account package. */
    readonly accessToken: string;
    /** When the access token expires, in UTC epoch milliseconds. */
    readonly expiresAt: number;
    /** The refresh token, null for a client that asked none, such as an installation's origin. */
    readonly refreshToken: string | null;
}

/** The tokens of a sign-in. */
export const TokenSet = {
    /** Read the refresh token a device keeps, refusing tokens without one. */
    refreshToken(tokens: TokenSet): string {
        if (tokens.refreshToken === null) {
            throw AccountError.of(
                "REFRESH_TOKEN_UNREACHABLE",
                "could not read a refresh token from the issuer",
            );
        }

        return tokens.refreshToken;
    },
};

/** A completed sign-in: the client's tokens and the person of its ID token. */
export interface SignedIn {
    /** The client's tokens. */
    readonly tokens: TokenSet;
    /** The signed-in person's user. */
    readonly userId: Identifier<"user">;
}

/** The outcome of polling a device code. */
export type DevicePoll =
    | { readonly status: "granted"; readonly signedIn: SignedIn }
    | { readonly status: "pending" | "slow" | "refused" };

/** What a person's package token acts in: a partition, a delegate's grant, both, or neither. */
export interface TokenBinding {
    /** The partition the token acts in, absent toward a universe package. */
    readonly partition?: PartitionId;
    /** The person the token acts as, or for under their grants to the caller. */
    readonly subject?: Subject;
    /** The chain of grants the token acts under. */
    readonly grants?: readonly Identifier<"grant">[];
    /** The grant the calling installation asks its subject for, refused until a grant gives it. */
    readonly requested?: GrantRequest;
    /** The person's session or client the token lives within, refused once it ends. */
    readonly credential?: Identifier<"session"> | Identifier<"client">;
}

/** The issuer a native device signs people in with, by redirect or device code. */
export class Issuer {
    /** The issuer's URL, the account package's oauth mount. */
    readonly url: string;
    /** The fetch calling the issuer. */
    readonly #fetch: Fetch;
    /** The environment the calls run in. */
    readonly #environment: Environment;
    /** The positions the caller's calls saw, which the issuer's answers add to. */
    readonly #bookmark: Bookmark;

    /** Call an issuer through a fetch, in an environment, keeping the positions it answers. */
    constructor(url: string, fetch: Fetch, environment: Environment, bookmark: Bookmark) {
        // store the issuer's URL, the fetch, the environment and the bookmark
        this.url = url.replace(/\/$/u, "");
        this.#fetch = fetch;
        this.#environment = environment;
        this.#bookmark = bookmark;
    }

    /** Start a browser sign-in returning to a client's redirect, fresh for a step-up. */
    async authorize(
        clientId: string,
        redirect: string,
        state: string,
        scopes: readonly string[],
        now: number,
        stepUp?: StepUp,
    ): Promise<BrowserAuthorization> {
        // ask with the parameters in the address
        const { parameters, verifier } = await this.#authorization(
            clientId,
            redirect,
            state,
            scopes,
            stepUp,
        );
        const url = new URL(`${this.url}${AUTHORIZATION_PATH}`);
        url.search = new URLSearchParams(parameters).toString();

        return { url: url.href, verifier, expiresAt: now + BROWSER_MILLISECONDS };
    }

    /** Start a browser sign-in whose parameters the client signs as a request object (RFC 9101). */
    async authorizeSigned(
        clientId: string,
        redirect: string,
        state: string,
        scopes: readonly string[],
        now: number,
        sign: (parameters: Readonly<Record<string, string>>) => Promise<string>,
        stepUp?: StepUp,
    ): Promise<BrowserAuthorization> {
        // ask with the signed parameters, beside the client they name
        const { parameters, verifier } = await this.#authorization(
            clientId,
            redirect,
            state,
            scopes,
            stepUp,
        );
        const url = new URL(`${this.url}${AUTHORIZATION_PATH}`);
        url.search = new URLSearchParams({
            client_id: clientId,
            request: await sign(parameters),
        }).toString();

        return { url: url.href, verifier, expiresAt: now + BROWSER_MILLISECONDS };
    }

    /** Write a code flow's parameters and the PKCE verifier its S256 challenge commits to. */
    async #authorization(
        clientId: string,
        redirect: string,
        state: string,
        scopes: readonly string[],
        stepUp?: StepUp,
    ): Promise<{ readonly parameters: Record<string, string>; readonly verifier: string }> {
        const verifier = Pkce.verifier(this.#environment.random);
        const parameters = {
            response_type: "code",
            client_id: clientId,
            redirect_uri: redirect,
            state,
            scope: scopes.join(" "),
            resource: PackageId.resource(accountPackage.id),
            code_challenge: await Pkce.challenge(verifier, this.#environment.crypto),
            code_challenge_method: "S256",
            ...(stepUp === undefined ? {} : { acr_values: `aal${stepUp.assurance}` }),
            ...(stepUp?.maxAge === undefined
                ? {}
                : {
                      prompt: "login",
                      max_age: String(Math.floor(stepUp.maxAge / SECOND_MILLISECONDS)),
                  }),
        };

        return { parameters, verifier };
    }

    /** Redeem a redirect's authorization code for the person's tokens. */
    async redeem(
        clientId: string,
        code: string,
        verifier: string,
        redirect: string,
    ): Promise<SignedIn> {
        return this.#signedIn(await this.#code(clientId, code, verifier, redirect));
    }

    /** Post an authorization code with its verifier to the token endpoint (RFC 6749 4.1.3). */
    async #code(
        clientId: string,
        code: string,
        verifier: string,
        redirect: string,
    ): Promise<TokenResponse> {
        const response = await this.#post(TOKEN_PATH, {
            grant_type: "authorization_code",
            code,
            redirect_uri: redirect,
            client_id: clientId,
            code_verifier: verifier,
        });
        if (!response.ok) {
            throw await TokenResponse.refusal(response);
        }

        return Issuer.#read(response, TokenResponse);
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
            throw AccountError.of(
                "MAIL_UNREACHABLE",
                `could not get the issuer to mail a code: status ${response.status}`,
            );
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
            throw await TokenResponse.refusal(response);
        }

        return this.#signedIn(await Issuer.#read(response, TokenResponse));
    }

    /** Start a device sign-in of a client. */
    async device(clientId: Identifier<"client">, now: number): Promise<DeviceAuthorization> {
        // ask the issuer for the codes
        const response = await this.#post(DEVICE_AUTHORIZATION_PATH, {
            client_id: clientId,
            scope: DEVICE_SCOPES.join(" "),
            resource: PackageId.resource(accountPackage.id),
        });
        if (!response.ok) {
            throw AccountError.of(
                "DEVICE_CODE_UNREACHABLE",
                `could not get a device code from the issuer: status ${response.status}`,
            );
        }

        // show the page with the code filled in and poll with the device code
        const device = await Issuer.#read(response, DeviceResponse);

        return {
            url: device.verification_uri_complete ?? device.verification_uri,
            userCode: device.user_code,
            deviceCode: device.device_code,
            interval: (device.interval ?? DEVICE_INTERVAL_SECONDS) * SECOND_MILLISECONDS,
            expiresAt: now + device.expires_in * SECOND_MILLISECONDS,
        };
    }

    /** Poll a device code once. */
    async poll(clientId: Identifier<"client">, deviceCode: string): Promise<DevicePoll> {
        // take the granted tokens
        const response = await this.#post(TOKEN_PATH, {
            grant_type: DEVICE_CODE,
            device_code: deviceCode,
            client_id: clientId,
        });
        if (response.ok) {
            return {
                status: "granted",
                signedIn: this.#signedIn(await Issuer.#read(response, TokenResponse)),
            };
        }

        // wait while the person decides
        const body = await response.text();
        const { error } = await Issuer.#read(new Response(body, response), ErrorResponse);
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
        // fail as the issuer refused on any other error
        else {
            throw await TokenResponse.refusal(new Response(body, response));
        }
    }

    /** Refresh a client's tokens with its refresh token. */
    refresh(clientId: Identifier<"client">, refreshToken: string): Promise<TokenSet> {
        return this.#tokens({
            grant_type: "refresh_token",
            refresh_token: refreshToken,
            client_id: clientId,
        });
    }

    /** Exchange a person's token for one of a package, in a space or under a grant (RFC 8693). */
    async exchange(
        subject: SubjectToken,
        audience: PackageId,
        bound: TokenBinding,
    ): Promise<{ readonly accessToken: string; readonly expiresAt: number }> {
        // post the person's token for the package's resource
        const response = await this.#post(TOKEN_PATH, {
            grant_type: TOKEN_EXCHANGE,
            subject_token: subject.token,
            subject_token_type: subject.type,
            resource: PackageId.resource(audience),
            ...(bound.partition === undefined ? {} : { partition: bound.partition }),
            ...(bound.subject === undefined ? {} : { subject: Subject.key(bound.subject) }),
            ...(bound.grants === undefined ? {} : { grants: bound.grants.join(" ") }),
            ...(bound.credential === undefined ? {} : { credential: bound.credential }),
            ...(bound.requested === undefined
                ? {}
                : {
                      authorization_details: JSON.stringify([
                          { ...bound.requested, type: GRANT_DETAIL },
                      ]),
                  }),
        });
        if (!response.ok) {
            throw await TokenResponse.refusal(response);
        }

        return TokenResponse.granted(
            await Issuer.#read(response, TokenResponse),
            this.#environment.clock.now(),
        );
    }

    /** Revoke a client's refresh token at the issuer (RFC 7009). */
    async revoke(clientId: Identifier<"client">, refreshToken: string): Promise<void> {
        // post the refresh token
        const response = await this.#post(REVOCATION_PATH, {
            token: refreshToken,
            token_type_hint: "refresh_token",
            client_id: clientId,
        });
        if (!response.ok) {
            throw AccountError.of(
                "REVOCATION_UNREACHABLE",
                `could not revoke the token at the issuer: status ${response.status}`,
            );
        }

        // read the answer to its end, which RFC 7009 leaves without content
        await response.arrayBuffer();
    }

    /** List the residencies a new person's account may store data in. */
    async residencies(): Promise<Pick<Residency, "code" | "name">[]> {
        const items = await Array.fromAsync(ListPage.all(this.accounts().residency.list, {}));

        return items.map(({ code, name }) => ({ code, name }));
    }

    /** Connect to the account package at the issuer, with an access token when given. */
    accounts(accessToken?: string) {
        return createClient(
            accountPackage,
            {
                url: PackageMount.url(new URL(this.url).origin, accountPackage.id),
                ...(accessToken === undefined
                    ? {}
                    : { headers: { authorization: `Bearer ${accessToken}` } }),
                fetch: this.#fetch,
            },
            this.#environment,
        );
    }

    /** Post a token request, refusing a refused one as the issuer refused it. */
    async #tokens(form: Readonly<Record<string, string>>): Promise<TokenSet> {
        const response = await this.#post(TOKEN_PATH, form);
        if (!response.ok) {
            throw await TokenResponse.refusal(response);
        }

        return this.#granted(await Issuer.#read(response, TokenResponse));
    }

    /** Read granted tokens, refusing a grant without a refresh token. */
    #granted(tokens: TokenResponse): TokenSet {
        const now = this.#environment.clock.now();

        return {
            ...TokenResponse.granted(tokens, now),
            refreshToken: tokens.refresh_token ?? null,
        };
    }

    /** Read a sign-in's tokens and the person of its ID token. */
    #signedIn(granted: TokenResponse): SignedIn {
        return { tokens: this.#granted(granted), userId: Issuer.#subject(granted) };
    }

    /** Read the person of a token response's ID token. */
    static #subject(granted: TokenResponse): Identifier<"user"> {
        // refuse a sign-in without an ID token
        if (granted.id_token === undefined) {
            throw AccountError.of(
                "ID_TOKEN_UNREACHABLE",
                "could not read an ID token from the issuer",
            );
        }

        // refuse an ID token without a payload
        const [, payload] = granted.id_token.split(".");
        if (payload === undefined) {
            throw AccountError.of("ID_TOKEN_UNREACHABLE", "could not parse the issuer's ID token");
        }

        // read the person from its claims, trusted as the token endpoint's answer (OIDC Core 3.1.3.7)
        const claims = new TextDecoder().decode(
            Uint8Array.fromBase64(payload, { alphabet: "base64url" }),
        );
        const { sub } = IdClaims.parse(JSON.parse(claims));

        return sub;
    }

    /** Post a form to one of the issuer's endpoints, keeping the position it answers. */
    async #post(path: string, form: Readonly<Record<string, string>>): Promise<Response> {
        // post the form, then keep each position the answer reports
        const response = await this.#fetch(
            new Request(`${this.url}${path}`, {
                method: "POST",
                headers: { "content-type": "application/x-www-form-urlencoded" },
                body: new URLSearchParams(form),
            }),
        );
        for (const watermark of Bookmark.parse(response.headers.get(BOOKMARK_HEADER)).watermarks) {
            this.#bookmark.observe(watermark);
        }

        return response;
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
            throw AccountError.of(
                "ANSWER_UNREACHABLE",
                `could not read the issuer's answer: status ${response.status}`,
            );
        }

        return parsed.data;
    }
}

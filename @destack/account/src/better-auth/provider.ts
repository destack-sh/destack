import type { BetterAuthPlugin } from "better-auth";
import type { OAuth2Tokens } from "better-auth/oauth2";
import {
    type GenericOAuthConfig,
    genericOAuth,
    type GenericOAuthUserInfo,
} from "better-auth/plugins";
import { getOAuthState } from "better-auth/api";
import type { Environment } from "@destack/environment";
import { Jwt } from "@destack/identity";
import { present, schema } from "@destack/schema";
import { AccountError } from "../error/error.ts";
import { SignUp } from "./sign-up.ts";
import { SECOND_MILLISECONDS } from "../authentication/token.ts";

/** The user agent GitHub's API requires on every request. */
const USER_AGENT = "Destack";

/** The issuers Google's ID tokens name, as its OpenID Connect discovery document lists them. */
const GOOGLE_ISSUERS = new Set(["https://accounts.google.com", "accounts.google.com"]);

/** A provider's answer to an authorization code exchange (RFC 6749 5.1). */
const TokenAnswer = schema.looseObject({
    access_token: schema.string(),
    token_type: schema.string().exactOptional(),
    scope: schema.string().exactOptional(),
    expires_in: schema.number().exactOptional(),
    refresh_token: schema.string().exactOptional(),
    id_token: schema.string().exactOptional(),
});

/** A GitHub user as its API answers it. */
const GitHubUser = schema.looseObject({
    id: schema.number(),
    login: schema.string(),
    name: schema.string().nullable(),
    email: schema.string().nullable(),
    avatar_url: schema.string().exactOptional(),
});

/** A GitHub user's email addresses as its API answers them. */
const GitHubEmails = schema.array(
    schema.looseObject({
        email: schema.string(),
        primary: schema.boolean(),
        verified: schema.boolean(),
    }),
);

/** The claims of a Google ID token a sign-in reads (OpenID Connect Core 5.1). */
const GoogleClaims = schema.looseObject({
    iss: schema.string(),
    aud: schema.string(),
    exp: schema.number(),
    sub: schema.string(),
    email: schema.string().exactOptional(),
    email_verified: schema.boolean().exactOptional(),
    name: schema.string().exactOptional(),
    picture: schema.string().exactOptional(),
});

/** The OAuth providers people sign in with. */
const PROVIDER_IDS = ["github", "google"] as const;

/** An OAuth provider people sign in with. */
export type ProviderId = (typeof PROVIDER_IDS)[number];

/** A deployment's application at a provider. */
export interface ProviderClient {
    /** The application's client identifier. */
    readonly clientId: string;
    /** The application's client secret. */
    readonly clientSecret: string;
}

/** The effects a provider is called with: its network, and the clock its tokens expire on. */
export type ProviderEnvironment = Pick<Environment, "network" | "clock">;

/** A provider's endpoints, scopes and profile. */
interface Provider {
    /** The name the sign-in page shows. */
    readonly name: string;
    /** The authorization endpoint people are sent to. */
    readonly authorization: string;
    /** The token endpoint codes are exchanged at. */
    readonly token: string;
    /** The scopes a sign-in asks for. */
    readonly scopes: readonly string[];
    /** Read the profile of the person the tokens were issued to. */
    readonly profile: (
        tokens: OAuth2Tokens,
        client: ProviderClient,
        environment: ProviderEnvironment,
    ) => Promise<GenericOAuthUserInfo>;
}

/** Each provider people sign in with. */
const PROVIDERS: Readonly<Record<ProviderId, Provider>> = {
    github: {
        name: "GitHub",
        authorization: "https://github.com/login/oauth/authorize",
        token: "https://github.com/login/oauth/access_token",
        scopes: ["read:user", "user:email"],
        profile: async (tokens, _client, environment) => {
            // read the user, whose public email GitHub shows only once verified
            const read = (path: string) => github(path, tokens, environment);
            const person = GitHubUser.parse(await read("/user"));
            const profile = { ...person, name: person.name ?? undefined, image: person.avatar_url };
            if (person.email !== null) {
                return { ...profile, email: person.email, emailVerified: true };
            }

            // read the primary address of a profile keeping its email private
            const emails = GitHubEmails.parse(await read("/user/emails"));
            const primary = emails.find((entry) => entry.primary);

            return {
                ...profile,
                email: primary?.email ?? null,
                emailVerified: primary?.verified ?? false,
            };
        },
    },
    google: {
        name: "Google",
        authorization: "https://accounts.google.com/o/oauth2/v2/auth",
        token: "https://oauth2.googleapis.com/token",
        scopes: ["openid", "email", "profile"],
        // read the ID token the token endpoint answered over TLS (OpenID Connect Core 3.1.3.7)
        profile: async (tokens, client, environment) => {
            // read the ID token's claims, requiring Google as issuer, this client as audience and a live token
            const idToken = present(tokens.idToken, "google's id token");
            const claims = GoogleClaims.parse(Jwt.claims(idToken));
            const now = environment.clock.now();
            if (!GOOGLE_ISSUERS.has(claims.iss)) {
                throw AccountError.of(
                    "TOKEN_INVALID",
                    `google's id token names the issuer ${claims.iss}`,
                );
            } else if (claims.aud !== client.clientId) {
                throw AccountError.of("TOKEN_INVALID", `google's id token is for ${claims.aud}`);
            } else if (claims.exp * SECOND_MILLISECONDS <= now) {
                throw AccountError.of("TOKEN_INVALID", "google's id token expired");
            }

            return {
                ...claims,
                id: claims.sub,
                emailVerified: claims.email_verified ?? false,
                image: claims.picture,
            };
        },
    },
};

/** The providers a deployment signs people in through. */
export const SignInProvider = {
    /** Sign people in through each configured provider, calling it on the environment's network. */
    plugin(
        clients: Readonly<Partial<Record<ProviderId, ProviderClient>>>,
        environment: ProviderEnvironment,
    ): BetterAuthPlugin {
        const configurations = PROVIDER_IDS.flatMap((id): GenericOAuthConfig<ProviderId>[] => {
            // leave out a provider the deployment has no application at
            const client = clients[id];
            if (client === undefined) {
                return [];
            }

            // exchange codes and read profiles through the environment
            const provider = PROVIDERS[id];

            return [
                {
                    providerId: id,
                    name: provider.name,
                    clientId: client.clientId,
                    clientSecret: client.clientSecret,
                    authorizationUrl: provider.authorization,
                    scopes: [...provider.scopes],
                    getToken: (data) => exchange(id, client, data, environment),
                    getUserInfo: (tokens) => provider.profile(tokens, client, environment),
                    mapProfileToUser: async (profile) => {
                        // read the callback's sign-up state and GitHub's login
                        const state = present(
                            await getOAuthState(),
                            "the OAuth state of the callback",
                        );
                        const login = id === "github" ? { login: profile["login"] } : {};

                        // pass a text name on, leaving any other for the sign-up to refuse as missing
                        const { name, ...claimed } = SignUp.claimed(profile, state);
                        const named = typeof name === "string" ? { name } : {};

                        return { ...login, ...claimed, ...named };
                    },
                },
            ];
        });

        return genericOAuth({ config: configurations });
    },
};

/** Exchange an authorization code for tokens at a provider's token endpoint (RFC 6749 4.1.3). */
async function exchange(
    id: ProviderId,
    client: ProviderClient,
    data: {
        readonly code: string;
        readonly redirectURI: string;
        readonly codeVerifier?: string | undefined;
    },
    environment: ProviderEnvironment,
): Promise<OAuth2Tokens> {
    // post the code with the client's credentials and the sign-in's verifier
    const body = new URLSearchParams({
        grant_type: "authorization_code",
        code: data.code,
        redirect_uri: data.redirectURI,
        client_id: client.clientId,
        client_secret: client.clientSecret,
        ...(data.codeVerifier === undefined ? {} : { code_verifier: data.codeVerifier }),
    });
    const headers = { accept: "application/json" };
    const request = new Request(PROVIDERS[id].token, { method: "POST", headers, body });
    const response = await environment.network.fetch(request);
    if (!response.ok) {
        throw AccountError.of(
            "TOKEN_ENDPOINT_FAILED",
            `${id} answered the code exchange with ${response.status}`,
        );
    }

    // read the tokens, timing their expiry on the environment's clock
    const answer = TokenAnswer.parse(await response.json());
    const expiresAt =
        answer.expires_in === undefined
            ? {}
            : {
                  accessTokenExpiresAt: new Date(
                      environment.clock.now() + answer.expires_in * SECOND_MILLISECONDS,
                  ),
              };

    return {
        accessToken: answer.access_token,
        tokenType: answer.token_type,
        refreshToken: answer.refresh_token,
        idToken: answer.id_token,
        scopes: answer.scope?.split(/[ ,]/u).filter((scope) => scope !== "") ?? [],
        ...expiresAt,
        raw: answer,
    };
}

/** Read a path of GitHub's API with an access token. */
async function github(
    path: string,
    tokens: OAuth2Tokens,
    environment: ProviderEnvironment,
): Promise<unknown> {
    // call the API as the token's user
    const accessToken = present(tokens.accessToken, "github's access token");
    const headers = {
        accept: "application/vnd.github+json",
        authorization: `Bearer ${accessToken}`,
        "user-agent": USER_AGENT,
    };
    const response = await environment.network.fetch(
        new Request(`https://api.github.com${path}`, { headers }),
    );
    if (!response.ok) {
        throw AccountError.of(
            "ANSWER_UNREADABLE",
            `github answered ${path} with ${response.status}`,
        );
    }

    return response.json();
}

import { betterAuth, type BetterAuthOptions } from "better-auth/minimal";
import { Authorizer } from "@destack/access";
import { and, eq, type DatabaseConnection } from "@destack/db";
import { magicLink, emailOTP, bearer, jwt } from "better-auth/plugins";
import { passkey } from "@better-auth/passkey";
import { oauthDeviceAuthorization, oauthProvider } from "@better-auth/oauth-provider";
import { v7 } from "uuid";
import { ServiceError } from "@destack/service/error";
import { authenticationDatabase } from "./database.ts";
import { authorizeSignIn, sessionPolicy } from "./session.ts";
import { secondFactor } from "./factor.ts";
import { nativeAuthorization } from "./native.ts";
import { APIError, isAPIError } from "better-auth/api";
import { AuthenticationAudit } from "./audit.ts";
import { SignInPages } from "./page.ts";
import { user } from "../object/user.ts";
import { account } from "../object/account.ts";
import { Digest } from "../object/digest.ts";
import { identifier } from "@destack/schema";

/** Signing-key rotation interval in seconds. */
const SIGNING_KEY_ROTATION_SECONDS = 7 * 24 * 60 * 60;
/** Retain expired public keys beyond the access-token lifetime, measured in seconds. */
const SIGNING_KEY_GRACE_SECONDS = 120;

/** How long an OAuth access token lives, ten minutes in seconds. */
const ACCESS_TOKEN_SECONDS = 10 * 60;

/** The path Better Auth's routes answer under. */
const BASE_PATH = "/auth";

/** The path the universe's token signing keys answer at, the JWT plugin's key set. */
export const JWKS_PATH = `${BASE_PATH}/jwks`;

/** The provider's discovery documents at the issuer's root. */
const DISCOVERY = new Set([
    "/.well-known/openid-configuration",
    "/.well-known/oauth-authorization-server",
]);

/** The Better Auth routes the account service serves as objects. */
const OBJECT_ROUTES = [
    "/token",
    "/update-user",
    "/list-sessions",
    "/revoke-session",
    "/revoke-sessions",
    "/revoke-other-sessions",
    "/oauth2/get-consent",
    "/oauth2/get-consents",
    "/oauth2/update-consent",
    "/oauth2/delete-consent",
];

/** The claims the provider supplies, per OpenID Connect Core 5.1. */
const CLAIMS = [
    "sub",
    "iss",
    "aud",
    "exp",
    "iat",
    "sid",
    "scope",
    "azp",
    "name",
    "picture",
    "given_name",
    "family_name",
    "email",
    "email_verified",
    "preferred_username",
    "locale",
    "zoneinfo",
];

/** The native client Destack ships, the daemon. */
const NATIVE_CLIENT = "destack-daemon";

/** The identifier prefixes of Better Auth's models. */
const PREFIXES: Readonly<Record<string, string>> = {
    user: "user",
    account: "identity",
    session: "session",
    verification: "verification",
    rateLimit: "rate-limit",
    deviceCode: "device-authorization",
    passkey: "passkey",
    twoFactor: "two-factor",
    jwks: "signing-key",
    oauthClient: "oauth-client",
    oauthConsent: "oauth-consent",
    oauthResource: "oauth-resource",
    oauthClientResource: "oauth-client-resource",
    oauthRefreshToken: "oauth-refresh-token",
    oauthAccessToken: "oauth-access-token",
};

/** Deployment inputs for Destack platform sign-in. */
export interface AuthenticationOptions {
    /** The canonical platform origin, including its scheme. */
    origin: string;
    /** Exact browser origins allowed to use platform sessions. */
    trustedOrigins: string[];
    /** Proxy and address headers trusted by the deployment's HTTP ingress. */
    ipAddress: NonNullable<NonNullable<BetterAuthOptions["advanced"]>["ipAddress"]>;
    /** The deployment secret used to protect cookies and provider tokens. */
    secret: string;
    /** Versioned encryption keys, newest first, retained while older credentials remain in use. */
    secrets?: BetterAuthOptions["secrets"];
    /** The migrated global account database. */
    database: DatabaseConnection;
    /** The sign-in applications the deployment configures, such as GitHub, Google or Microsoft. */
    providers: NonNullable<BetterAuthOptions["socialProviders"]>;
    /** The browser page where users sign in. */
    signInUri: string;
    /** The browser page where users approve another application's access to their account. */
    consentUri: string;
    /** The browser page where users approve CLI and desktop sign-in. */
    verificationUri: string;
    /** The browser page where users complete a required second factor. */
    secondFactorUri: string;
    /** The browser page where users choose their handle after their first sign-in. */
    handleUri: string;
    /** Send a request to the account service in process, as the handle page acts on it. */
    service: (request: Request) => Promise<Response>;
    /** Deliver a sign-in link without retaining or logging its token. */
    sendMagicLink: (message: { email: string; url: string }) => Promise<void>;
    /** Deliver a one-use email code without retaining or logging its value. */
    sendCode: (message: {
        email: string;
        otp: string;
        type: "sign-in" | "email-verification" | "forget-password" | "change-email";
    }) => Promise<void>;
}

/** The configured platform authentication service. */
export type Authentication = ReturnType<typeof createAuthentication>;

/** Configure platform sessions and provider sign-in through Better Auth. */
export function createAuthentication(options: AuthenticationOptions) {
    // require the sign-in pages on the platform origin
    const origin = new URL(options.origin).origin;
    const signIn = new URL(options.signInUri, origin);
    const consent = new URL(options.consentUri, origin);
    const verification = new URL(options.verificationUri, origin);
    const challenge = new URL(options.secondFactorUri, origin);
    const handle = new URL(options.handleUri, origin);
    if ([signIn, consent, verification, challenge, handle].some((page) => page.origin !== origin)) {
        throw new ServiceError("PRECONDITION_FAILED", {
            message: "sign-in pages must use the platform origin",
        });
    }

    // configure Better Auth with audited sessions and the platform plugins
    const audit = new AuthenticationAudit(options.database);
    const users = new Authorizer([user.policy], [user.mapping]);
    const authentication = betterAuth({
        appName: "Destack",
        baseURL: options.origin,
        basePath: BASE_PATH,
        secret: options.secret,
        secrets: options.secrets,
        database: authenticationDatabase(options.database, users),
        trustedOrigins: options.trustedOrigins,
        socialProviders: withLogins(options.providers),
        telemetry: { enabled: false },
        disabledPaths: OBJECT_ROUTES,
        logger: { log: (level) => audit.log(level) },
        onAPIError: { onError: () => audit.log("error") },
        databaseHooks: {
            account: {
                create: {
                    before: async (identity) => ({ data: { ...identity, idToken: null } }),
                },
                update: {
                    before: async (identity) => ({ data: { ...identity, idToken: null } }),
                },
            },
            session: {
                create: {
                    before: async (session) => {
                        await authorizeSignIn(session.userId, options.database);

                        return { data: session };
                    },
                },
            },
        },

        // map Better Auth's models onto the account tables
        user: {
            modelName: "user",
            additionalFields: {
                login: { type: "string", input: false, required: false },
                locale: { type: "string", input: false, required: false },
                timeZone: { type: "string", input: false, required: false },
            },
        },
        account: {
            modelName: "identity",
            fields: { accountId: "providerUserId", scope: "providerScope" },
            encryptOAuthTokens: true,
            accountLinking: {
                enabled: true,
                disableImplicitLinking: true,
                allowDifferentEmails: false,
                allowUnlinkingAll: false,
            },
        },
        session: {
            modelName: "session",
            cookieCache: { enabled: false },
            additionalFields: {
                authenticatedAt: { type: "number", input: false, required: false },
                authenticationMethod: { type: "string", input: false, required: false },
            },
        },
        verification: { modelName: "verification" },
        advanced: {
            disableOriginCheck: false,
            disableCSRFCheck: false,
            database: { generateId: ({ model }) => generateIdentifier(model) },
            crossSubDomainCookies: { enabled: false },
            ipAddress: options.ipAddress,
        },
        rateLimit: { enabled: true, storage: "database" },

        // provide the sign-in plugins
        plugins: [
            jwt({
                schema: { jwks: { modelName: "signingKey" } },
                disableSettingJwtHeader: true,
                jwks: {
                    keyPairConfig: { alg: "ES256" },
                    rotationInterval: SIGNING_KEY_ROTATION_SECONDS,
                    gracePeriod: SIGNING_KEY_GRACE_SECONDS,
                },
                jwt: { issuer: options.origin, expirationTime: "60s" },
            }),
            bearer(),
            oauthProvider({
                loginPage: options.signInUri,
                consentPage: options.consentUri,
                grantTypes: ["authorization_code", "refresh_token"],
                accessTokenExpiresIn: ACCESS_TOKEN_SECONDS,
                storeClientSecret: { hash: (secret) => Digest.hex(secret) },
                clientPrivileges: () => false,
                resourcePrivileges: () => false,
                advertisedMetadata: { claims_supported: CLAIMS },
                customUserInfoClaims: ({ user, scopes }) =>
                    scopes.includes("profile") ? profileClaims(user, options.database) : {},
            }),
            passkey({
                rpID: new URL(options.origin).hostname,
                rpName: "Destack",
                origin: options.trustedOrigins,
                authenticatorSelection: { userVerification: "required" },
                registration: {
                    afterVerification: ({ verification }) => {
                        if (!verification.registrationInfo?.userVerified) {
                            throw new APIError("UNAUTHORIZED", {
                                message: "passkey user verification is required",
                            });
                        }
                    },
                },
                authentication: {
                    afterVerification: ({ verification }) => {
                        if (!verification.authenticationInfo.userVerified) {
                            throw new APIError("UNAUTHORIZED", {
                                message: "passkey user verification is required",
                            });
                        }
                    },
                },
            }),
            secondFactor(challenge.href),
            emailOTP({ storeOTP: "hashed", sendVerificationOTP: options.sendCode }),
            magicLink({
                storeToken: "hashed",
                sendMagicLink: ({ email, url }) => options.sendMagicLink({ email, url }),
            }),
            oauthDeviceAuthorization({
                schema: { deviceCode: { modelName: "deviceAuthorization" } },
                verificationUri: options.verificationUri,
                validateClient: (clientId) => clientId === NATIVE_CLIENT,
            }),
            nativeAuthorization({
                client: NATIVE_CLIENT,
                signInUri: options.signInUri,
                consentUri: options.consentUri,
            }),
            sessionPolicy(options.database),
            audit.plugin(),
        ],
    });

    // retain the authentication protocol handler and its declared endpoint names
    const handler = authentication.handler;
    const routes = Object.values(authentication.api)
        .map((endpoint) => endpoint.path)
        .filter((path): path is string => typeof path === "string");

    // answer the protocol through the audit
    const protocol = async (request: Request): Promise<Response> => {
        // answer the public discovery documents
        if (DISCOVERY.has(new URL(request.url).pathname)) {
            return handler(request);
        }

        // identify the current user from a presented session
        let current = null;
        if (request.headers.has("cookie") || request.headers.has("authorization")) {
            try {
                current = await authentication.api.getSession({
                    headers: request.headers,
                    query: { disableRefresh: true },
                });
            } catch (error) {
                // treat a revoked credential as no actor
                if (!isAPIError(error) || error.statusCode !== 401) {
                    throw error;
                }
            }
        }

        return audit.invoke(request, AuthenticationAudit.actor(current), routes, handler);
    };

    // serve the sign-in pages over the protocol
    const pages = new SignInPages({
        endpoint: `${origin}${BASE_PATH}`,
        signIn,
        consent,
        verification,
        secondFactor: challenge,
        handle,
        context: authentication.$context,
        send: protocol,
        service: options.service,
    });

    return {
        ...authentication,
        /** The authoritative database for sessions and account permissions. */
        database: options.database,
        /** Report whether sign-in answers a path. */
        handles(path: string): boolean {
            return (
                path === BASE_PATH ||
                path.startsWith(`${BASE_PATH}/`) ||
                DISCOVERY.has(path) ||
                pages.handles(path)
            );
        },
        /** Answer a sign-in page or protocol request. */
        async handler(request: Request): Promise<Response> {
            return pages.handles(new URL(request.url).pathname)
                ? pages.serve(request)
                : protocol(request);
        },
    };
}

/** Keep the provider login that suggests a new user's handle. */
function withLogins(
    providers: AuthenticationOptions["providers"],
): AuthenticationOptions["providers"] {
    // leave providers without logins as they are
    const github = providers.github;
    if (github === undefined) {
        return providers;
    }

    // add GitHub's login to the deployment's profile mapping
    return {
        ...providers,
        github: async () => {
            const options = typeof github === "function" ? await github() : github;

            return {
                ...options,
                mapProfileToUser: async (profile) => ({
                    ...(await options.mapProfileToUser?.(profile)),
                    login: profile.login,
                }),
            };
        },
    };
}

/** Read a user's handle, locale and time zone claims. */
async function profileClaims(
    person: { id: string; locale?: string | null; timeZone?: string | null },
    database: DatabaseConnection,
): Promise<Record<string, string | undefined>> {
    // read the handle of the user's personal account
    const [personal] = await database
        .select({ handle: account.table.handle })
        .from(account.table)
        .where(
            and(
                eq(account.table.scope, identifier("user").parse(person.id)),
                eq(account.table.kind, "personal"),
            ),
        );

    return {
        preferred_username: personal?.handle,
        locale: person.locale ?? undefined,
        zoneinfo: person.timeZone ?? undefined,
    };
}

/** Generate the identifier required by the shared authentication model. */
function generateIdentifier(model: string): string {
    const prefix = PREFIXES[model];
    if (!prefix) {
        throw new ServiceError("INTERNAL_SERVER_ERROR", {
            message: `unknown authentication model: ${model}`,
        });
    }

    return `${prefix}-${v7()}`;
}

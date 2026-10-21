import { betterAuth, type BetterAuthOptions } from "better-auth/minimal";
import { Journal } from "@destack/object/server";
import type { BetterAuthPlugin } from "better-auth";
import type { CallKey } from "@destack/runtime/request";
import type { Fetch } from "@destack/runtime";
import type { ObjectStore } from "@destack/object/server";
import { Authorizer } from "@destack/access";
import { and, eq, type DatabaseConnection, Snapshot } from "@destack/db";
import type { EventStore } from "@destack/event";
import type { PackageId } from "@destack/package";
import { magicLink, emailOTP, jwt } from "better-auth/plugins";
import { passkey } from "@better-auth/passkey";
import { oauthDeviceAuthorization, oauthProvider } from "@better-auth/oauth-provider";
import type { IdentityKeystore } from "../identity/index.ts";
import type { Keyring } from "@destack/identity";
import { CallError } from "@destack/runtime/error";
import { Scope } from "@destack/sync";
import { betterAuthAdapter } from "./adapter.ts";
import { betterAuthSessionPolicy } from "./session-policy.ts";
import { betterAuthSecondFactor } from "./second-factor.ts";
import { APIError, getOAuthState, isAPIError } from "better-auth/api";
import { BetterAuthAudit } from "./audit.ts";
import { SignInPage } from "./sign-in-page.ts";
import { AccountMail, type MailContent, type MailTransport } from "./account-mail.ts";
import { JWKS_PATH, TOKEN_PATH } from "../authentication/path.ts";
import { Client } from "../object/client.ts";
import { Identity } from "../object/identity.ts";
import type { CodeCheck } from "../oauth/code.ts";
import { oauthExtension } from "../oauth/oauth.ts";
import { user, User } from "../object/user.ts";
import { account } from "../object/account.ts";
import { SignUp } from "./sign-up.ts";
import { Digest, Identifier, present, schema } from "@destack/schema";
import type { KeySet } from "@destack/identity";
import type { Clock } from "@destack/environment";

/** The sign-in pages below the issuer's URL. */
const SIGN_IN_PATHS = {
    signIn: "/sign-in",
    signUp: "/sign-up",
    consent: "/consent",
    verification: "/device",
    secondFactor: "/sign-in/two-factor",
} as const;

/** The label that derives the cookie and provider token secret from the process's root key. */
const SESSION_SECRET_LABEL = { label: "@destack/account/session-secret", context: "" } as const;

/** The prefix of a mailed sign-in code's verification identifier, Better Auth's email OTP's. */
const SIGN_IN_CODE = "sign-in-otp-";

/** The code Better Auth refuses a valid sign-in code of an email no person has with. */
const USER_NOT_FOUND = "USER_NOT_FOUND";

/** How long an OAuth access token lives, ten minutes in seconds. */
const ACCESS_TOKEN_SECONDS = 10 * 60;

/** How long a Better Auth JWT lives, a minute: one exchange with clock skew to spare. */
const JWT_LIFETIME = "60s";

/** The provider's discovery documents at the issuer's root. */
const DISCOVERY = new Set([
    "/.well-known/openid-configuration",
    "/.well-known/oauth-authorization-server",
]);

/** The Better Auth routes the account package serves as objects. */
const OBJECT_ROUTES = [
    "/device/token",
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

/** The identifier prefixes of Better Auth's models. */
const MODEL_PREFIXES: Readonly<Record<string, string>> = {
    user: "user",
    account: "user-login",
    session: "session",
    verification: "verification",
    rateLimit: "rate-limit",
    deviceCode: "device-authorization",
    passkey: "passkey",
    twoFactor: "two-factor",
    oauthClient: "oauth-client",
    oauthConsent: "oauth-consent",
    oauthResource: "oauth-resource",
    oauthClientResource: "oauth-client-resource",
    oauthRefreshToken: "oauth-refresh-token",
    oauthAccessToken: "oauth-access-token",
};

/** Better Auth's models mapped onto the account tables. */
const MODELS = {
    user: {
        modelName: "user",
        additionalFields: {
            login: { type: "string", input: false, required: false },
            timeZone: { type: "string", input: true, required: true },
            locale: { type: "string", input: true, required: true },
            // pass the personal account's handle and residency to the adapter
            handle: { type: "string", input: true, required: true, returned: false },
            residencyId: { type: "string", input: true, required: true, returned: false },
        },
    },
    account: {
        modelName: "userLogin",
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
} satisfies Pick<BetterAuthOptions, "user" | "account" | "session" | "verification">;

/** Deployment inputs for Destack platform sign-in. */
export interface BetterAuthAuthenticatorOptions {
    /** The issuer's URL: the oauth adapter's, below the account package's mount. */
    issuer: string;
    /** Exact browser origins allowed to use platform sessions. */
    trustedOrigins: string[];
    /** Proxy and address headers trusted by the deployment's HTTP ingress. */
    ipAddress: NonNullable<NonNullable<BetterAuthOptions["advanced"]>["ipAddress"]>;
    /** The universe's keystore, whose identity signs the universe's tokens. */
    keystore: IdentityKeystore;
    /** The migrated account database. */
    database: DatabaseConnection;
    /** Read the deployment's key that sensitive call inputs are fingerprinted under. */
    callKey: CallKey;
    /** The sign-in applications the deployment configures, such as GitHub, Google or Microsoft. */
    providers: NonNullable<BetterAuthOptions["socialProviders"]>;
    /** The account objects, which create a new person's personal account and decide access. */
    objects: Pick<ObjectStore, "mutate" | "claims" | "authorizer" | "database" | "serving">;
    /** The event log the journal stores audited calls in. */
    events: EventStore;
    /** The lifetimes of the packages' access tokens, in milliseconds, a minute when absent. */
    lifetimes: ReadonlyMap<PackageId, number>;
    /** The clock sign-in reads time at. */
    clock: Clock;
    /** The transport mailing sign-in links and codes. */
    mail: MailTransport;
}

/** The sign-in applications a deployment configures. */
type Providers = BetterAuthAuthenticatorOptions["providers"];

/** The universe's sign-in through Better Auth, answering below the issuer's URL. */
export class BetterAuthAuthenticator {
    /** The Better Auth instance storing sessions, codes and provider sign-ins. */
    readonly auth: Authentication;
    /** The authoritative database for sessions and account permissions. */
    readonly database: DatabaseConnection;
    /** The clock sign-in reads time at. */
    readonly clock: Clock;
    /** The keys of the universe's identity, which sign its tokens. */
    readonly keystore: IdentityKeystore;
    /** The issuer's URL, below which every request is answered. */
    readonly issuer: string;
    /** The sign-in pages, answered before the protocol. */
    readonly #pages: SignInPage;
    /** The audited Better Auth protocol. */
    readonly #protocol: Fetch;

    /** Answer sign-in through a Better Auth instance, its pages and its audited protocol. */
    private constructor(
        options: BetterAuthAuthenticatorOptions,
        auth: Authentication,
        pages: SignInPage,
        protocol: Fetch,
    ) {
        // store the deployment's database, clock, keys and issuer
        this.database = options.database;
        this.clock = options.clock;
        this.keystore = options.keystore;
        this.issuer = options.issuer;

        // answer through the instance, its pages and its protocol
        this.auth = auth;
        this.#pages = pages;
        this.#protocol = protocol;
    }

    /** Configure platform sessions and provider sign-in through Better Auth at the issuer's URL. */
    static async create(options: BetterAuthAuthenticatorOptions): Promise<BetterAuthAuthenticator> {
        // audit the protocol below the issuer's URL
        const { keystore } = options;
        const issuer = new URL(options.issuer);
        const pages = resolvePages(options.issuer);
        const audit = new BetterAuthAudit(
            new Journal(options.database, options.callKey, {
                events: options.events,
                clock: options.clock,
            }),
            issuer.pathname,
        );

        // check and spend a mailed sign-in code for the email code grant
        const codes: CodeCheck = async (email, code) => {
            // check the code, accepting it for an email nobody signed up with yet
            await auth.api
                .checkVerificationOTP({ body: { email, type: "sign-in", otp: code } })
                .catch((error: unknown) => {
                    // accept the refusal that follows a checked code: no such person yet
                    if (!isAPIError(error) || error.body?.code !== USER_NOT_FOUND) {
                        throw error;
                    }
                });

            // spend it
            const context = await auth.$context;
            await context.internalAdapter.deleteVerificationByIdentifier(
                `${SIGN_IN_CODE}${email.toLowerCase()}`,
            );
        };

        // configure Better Auth with audited sessions over the account tables
        const secrets = await sessionSecrets(keystore.keyring);
        const auth = configure(options, audit, pages, secrets, codes);

        // serve the sign-in pages over the audited protocol
        const protocol = auditedProtocol(auth, audit, issuer.pathname);
        const signIn = new SignInPage({
            ...pages,
            endpoint: options.issuer,
            context: auth.$context,
            send: protocol,
            database: options.database,
            register: async (request, clientId) => {
                // read the person the request's session signs in
                const current = await auth.api.getSession({ headers: request.headers });
                if (current === null) {
                    throw new CallError("UNAUTHORIZED", { message: "the person is not signed in" });
                }

                await Client.register(
                    options.objects,
                    clientId,
                    current.user.id,
                    options.clock.now(),
                );
            },
        });

        return new BetterAuthAuthenticator(options, auth, signIn, protocol);
    }

    /** Read the key set the universe's tokens verify against: its identity's signing keys. */
    keys(): Promise<KeySet> {
        return Identity.keys(Snapshot.live(this.database), Scope.universe.id);
    }

    /** Answer a request below the issuer: its key set, a sign-in page or a protocol request. */
    async handler(request: Request): Promise<Response> {
        // place the request below the issuer's URL, as Better Auth routes it
        const issuer = new URL(this.issuer);
        const below = new URL(request.url);
        const url = new URL(`${issuer.pathname}${below.pathname}${below.search}`, issuer);
        const placed = new Request(url.href, request);

        // answer the universe's key set
        if (below.pathname === JWKS_PATH) {
            return Response.json(await this.keys());
        }
        // answer a sign-in page
        else if (this.#pages.handles(url.pathname)) {
            return this.#pages.serve(placed);
        }
        // answer a protocol request
        else {
            return this.#protocol(placed);
        }
    }
}

/** The Better Auth instance of the universe's sign-in. */
type Authentication = ReturnType<typeof configure>;

/** Configure Better Auth with audited sessions over the account tables. */
function configure(
    options: BetterAuthAuthenticatorOptions,
    audit: BetterAuthAudit,
    pages: Pick<SignInPage, keyof typeof SIGN_IN_PATHS>,
    secrets: Awaited<ReturnType<typeof sessionSecrets>>,
    codes: CodeCheck,
) {
    return betterAuth({
        ...settings(options, audit, secrets),
        ...MODELS,
        plugins: plugins(options, audit, pages, codes),
    });
}

/** Answer Better Auth's protocol requests through the audit as the session's user. */
function auditedProtocol(
    authentication: Pick<ReturnType<typeof betterAuth<typeof MODELS>>, "api" | "handler">,
    audit: BetterAuthAudit,
    basePath: string,
): Fetch {
    // list the protocol's routes once
    const routes = Object.values(authentication.api)
        .map((endpoint) => endpoint.path)
        .filter((path): path is string => typeof path === "string");

    // read a session without refreshing it
    const session = (headers: Headers) =>
        authentication.api.getSession({ headers, query: { disableRefresh: true } });

    return async (request) => {
        // answer the public discovery documents
        if (DISCOVERY.has(new URL(request.url).pathname.slice(basePath.length))) {
            return authentication.handler(request);
        }

        // answer as the user of the presented session
        const current = await readSession(request, session);

        return audit.invoke(
            request,
            BetterAuthAudit.caller(current),
            routes,
            authentication.handler,
        );
    };
}

/** Derive the session secrets of every keyring version, newest first. */
async function sessionSecrets(keyring: Keyring) {
    // derive one secret per keyring version
    const derived = await keyring.deriveEach(SESSION_SECRET_LABEL);

    return derived.map(({ version, secret }) => ({ version, value: secret.toHex() }));
}

/** Resolve the sign-in pages below the issuer's URL. */
function resolvePages(issuer: string): Pick<SignInPage, keyof typeof SIGN_IN_PATHS> {
    const page = (path: string) => new URL(`${issuer}${path}`);

    return {
        signIn: page(SIGN_IN_PATHS.signIn),
        signUp: page(SIGN_IN_PATHS.signUp),
        consent: page(SIGN_IN_PATHS.consent),
        verification: page(SIGN_IN_PATHS.verification),
        secondFactor: page(SIGN_IN_PATHS.secondFactor),
    };
}

/** Configure Better Auth's deployment, storage, hooks and request protection. */
function settings(
    options: BetterAuthAuthenticatorOptions,
    audit: BetterAuthAudit,
    secrets: NonNullable<BetterAuthOptions["secrets"]>,
) {
    // decide the users' own records through the user policy
    const users = new Authorizer([user.policy], [user.mapping]);

    return {
        appName: "Destack",
        baseURL: new URL(options.issuer).origin,
        basePath: new URL(options.issuer).pathname,
        secrets,
        database: betterAuthAdapter(
            options.database,
            users,
            options.objects,
            options.lifetimes,
            options.clock,
        ),
        trustedOrigins: options.trustedOrigins,
        socialProviders: withProfiles(options.providers),
        telemetry: { enabled: false },
        disabledPaths: OBJECT_ROUTES,
        logger: { log: (level) => audit.log(level) },
        onAPIError: { onError: () => audit.log("error") },
        databaseHooks: {
            user: {
                create: {
                    before: async (person, context) => {
                        // read the sign-up
                        const signUp = SignUp.of(person, context);

                        return { data: { ...person, ...signUp } };
                    },
                },
            },
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
                        // refuse a sign-in the user may not make
                        if (!(await User.maySignIn(options.database, session.userId))) {
                            throw new APIError("UNAUTHORIZED", {
                                message: "user is no longer authorized",
                            });
                        }

                        return { data: session };
                    },
                },
            },
        },
        advanced: {
            disableOriginCheck: false,
            disableCSRFCheck: false,
            database: { generateId: ({ model }) => generateIdentifier(model) },
            crossSubDomainCookies: { enabled: false },
            ipAddress: options.ipAddress,
        },
        rateLimit: { enabled: true, storage: "database" },
    } satisfies BetterAuthOptions;
}

/** Build the sign-in plugins. */
function plugins(
    options: BetterAuthAuthenticatorOptions,
    audit: BetterAuthAudit,
    pages: Pick<SignInPage, keyof typeof SIGN_IN_PATHS>,
    codes: CodeCheck,
) {
    return [
        jwtPlugin(options),
        provider({
            loginPage: pages.signIn.href,
            consentPage: pages.consent.href,
            grantTypes: ["authorization_code", "refresh_token"],
            // let the packages be the resources and Destack's access decide who calls them
            enforcePerClientResources: false,
            extensions: [
                oauthExtension({
                    database: options.database,
                    objects: options.objects,
                    codes,
                    authorizer: options.objects.authorizer,
                    issuer: options.issuer,
                    endpoint: `${options.issuer}${TOKEN_PATH}`,
                    clock: options.clock,
                    lifetimes: options.lifetimes,
                    sign: (claims, header) =>
                        options.keystore.sign(options.database, Scope.universe.id, claims, header),
                }),
            ],
            accessTokenExpiresIn: ACCESS_TOKEN_SECONDS,
            storeClientSecret: { hash: (secret) => Digest.of(secret) },
            clientPrivileges: () => false,
            resourcePrivileges: () => false,
            advertisedMetadata: { claims_supported: CLAIMS },
            customUserInfoClaims: ({ user: person, scopes }) =>
                scopes.includes("profile") ? profileClaims(person, options.database) : {},
        }),
        passkeys(options),
        betterAuthSecondFactor(pages.secondFactor.href),
        emailOTP({
            storeOTP: "hashed",
            sendVerificationOTP: ({ email, otp, type }) =>
                mail(options, email, AccountMail.code(otp, type)),
        }),
        magicLink({
            storeToken: "hashed",
            sendMagicLink: ({ email, url, metadata }) =>
                mail(
                    options,
                    email,
                    AccountMail.link(metadata === undefined ? url : SignUp.link(url, metadata)),
                ),
        }),
        oauthDeviceAuthorization({
            schema: { deviceCode: { modelName: "deviceAuthorization" } },
            verificationUri: pages.verification.href,
            validateClient: (clientId) => Client.isAdmissible(options.database, clientId),
        }),
        betterAuthSessionPolicy(options.database, options.clock),
        audit.plugin(),
    ];
}

/** Build the JWT plugin that signs with the universe's identity and publishes its key set. */
function jwtPlugin(options: BetterAuthAuthenticatorOptions) {
    return jwt({
        disableSettingJwtHeader: true,
        jwks: {
            keyPairConfig: { alg: "ES256" },
            remoteUrl: `${options.issuer}${JWKS_PATH}`,
        },
        jwt: {
            issuer: options.issuer,
            expirationTime: JWT_LIFETIME,
            sign: (payload, header) =>
                options.keystore.sign(options.database, Scope.universe.id, payload, header),
        },
    });
}

/** Build the passkey plugin for the platform origin. */
function passkeys(options: BetterAuthAuthenticatorOptions) {
    return passkey({
        rpID: new URL(options.issuer).hostname,
        rpName: "Destack",
        origin: options.trustedOrigins,
        authenticatorSelection: { userVerification: "required" },
        registration: {
            afterVerification: ({ verification }) => {
                // refuse a registration without user verification
                if (verification.registrationInfo?.userVerified !== true) {
                    throw new APIError("UNAUTHORIZED", {
                        message: "passkey user verification is required",
                    });
                }
            },
        },
        authentication: {
            afterVerification: ({ verification }) => {
                // refuse a sign-in without user verification
                if (!verification.authenticationInfo.userVerified) {
                    throw new APIError("UNAUTHORIZED", {
                        message: "passkey user verification is required",
                    });
                }
            },
        },
    });
}

/** Read the session a request presents, none without credentials or for a revoked one. */
async function readSession<Session>(
    request: Request,
    read: (headers: Headers) => Promise<Session | null>,
): Promise<Session | null> {
    // read no session without credentials
    if (!request.headers.has("cookie") && !request.headers.has("authorization")) {
        return null;
    }

    // treat a revoked credential as no actor
    try {
        return await read(request.headers);
    } catch (error) {
        // rethrow any other failure
        if (!isAPIError(error) || error.statusCode !== 401) {
            throw error;
        }

        return null;
    }
}

/** Map every provider's profile onto a new user. */
function withProfiles(providers: Providers): Providers;
/**
 * Map every provider's profile onto a new user, its type stated by the signature above.
 *
 * NOTE #Architecture: Better Auth types providers by a map over its provider list.
 */
function withProfiles(providers: Providers): unknown {
    return Object.fromEntries(
        Object.entries(providers).map(([id, provider]) => [
            id,
            async () =>
                withProfile(id, typeof provider === "function" ? await provider() : provider),
        ]),
    );
}

/** Map a provider's profile onto a new user. */
function withProfile<Options extends { readonly mapProfileToUser?: unknown }>(
    id: string,
    options: Options,
) {
    // refuse a mapping of the deployment's own
    if (options.mapProfileToUser !== undefined) {
        throw new CallError("PRECONDITION_FAILED", {
            message: `the authenticator maps the profiles of provider ${id}`,
        });
    }

    return {
        ...options,
        mapProfileToUser: async (profile: object) => {
            // read the callback's sign-up state and GitHub's login
            const state = present(await getOAuthState(), "the OAuth state of the callback");
            const login = id === "github" && "login" in profile ? { login: profile.login } : {};

            return { ...login, ...SignUp.claimed(profile, state) };
        },
    };
}

/** Build the OAuth provider plugin as a plain plugin. */
function provider(options: Parameters<typeof oauthProvider>[0]): BetterAuthPlugin;
/**
 * Build the OAuth provider plugin, its type stated by the signature above.
 *
 * NOTE #Architecture: the plugin declares absent items as `items?: undefined`.
 */
function provider(options: Parameters<typeof oauthProvider>[0]): unknown {
    return oauthProvider(options);
}

/** Read a user's handle, locale and time zone claims. */
async function profileClaims(
    person: { readonly id: string; readonly timeZone?: unknown; readonly locale?: unknown },
    database: DatabaseConnection,
): Promise<Record<string, string | undefined>> {
    // read the time zone and locale of every user
    const scope = schema.identifier("user").parse(person.id);
    const { timeZone, locale } = SignUp.person.parse({
        timeZone: person.timeZone,
        locale: person.locale,
    });

    // read the handle of the user's personal account
    const [personal] = await database
        .select({ handle: account.table.handle })
        .from(account.table)
        .where(and(eq(account.table.scope, scope), eq(account.table.kind, "personal")));

    return {
        preferred_username: present(personal, `the personal account of ${scope}`).handle,
        locale,
        zoneinfo: timeZone,
    };
}

/** Generate the identifier required by the shared authentication model. */
function generateIdentifier(model: string): string {
    // refuse an unknown model
    const prefix = Object.hasOwn(MODEL_PREFIXES, model) ? MODEL_PREFIXES[model] : undefined;
    if (prefix === undefined) {
        throw new CallError("INTERNAL_SERVER_ERROR", {
            message: `unknown authentication model: ${model}`,
        });
    }

    return Identifier.create(prefix);
}

/** Mail a sign-in link or code to an address, refusing a message the transport did not send. */
async function mail(
    options: Pick<BetterAuthAuthenticatorOptions, "mail" | "clock">,
    address: string,
    content: MailContent,
): Promise<void> {
    // send the content to the address under a fresh key
    const outcome = await options.mail.send({
        ...content,
        to: address,
        key: crypto.randomUUID(),
        date: options.clock.now(),
    });

    // refuse a message the transport retried or failed
    if (outcome.kind !== "sent") {
        throw new CallError("UNAVAILABLE", {
            message: `sign-in mail to ${address} was not sent: ${outcome.error.message}`,
        });
    }
}

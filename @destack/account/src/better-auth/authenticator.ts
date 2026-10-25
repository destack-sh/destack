import { betterAuth, type BetterAuthOptions } from "better-auth/minimal";
import { Journal } from "@destack/object/server";
import type { CallKey } from "@destack/object";
import type { Fetch } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { type DatabaseConnection, Snapshot } from "@destack/db";
import type { EventStore } from "@destack/event";
import { magicLink, emailOTP } from "better-auth/plugins";
import { passkey } from "@better-auth/passkey";
import type { IdentityKeystore } from "../identity/index.ts";
import type { Keyring, KeySet } from "@destack/identity";
import { Scope } from "@destack/sync";
import { betterAuthAdapter } from "./adapter.ts";
import { betterAuthSessionPolicy } from "./session-policy.ts";
import { betterAuthSecondFactor } from "./second-factor.ts";
import { APIError, isAPIError } from "better-auth/api";
import { BetterAuthAudit } from "./audit.ts";
import { SignInPage } from "./sign-in-page.ts";
import { AccountMail, type MailContent, type MailTransport } from "./account-mail.ts";
import { JWKS_PATH } from "../authentication/path.ts";
import { Identity } from "../object/identity.ts";
import type { CodeCheck } from "../oauth/email.ts";
import type { TokenLifetime } from "../oauth/resource.ts";
import { AuthorizationServer } from "../oauth/server.ts";
import { User } from "../object/user.ts";
import { SignUp } from "./sign-up.ts";
import { type ProviderClient, type ProviderId, SignInProvider } from "./provider.ts";
import { Identifier, type UuidEnvironment, Uuid } from "@destack/schema";
import { type Clock, type Environment } from "@destack/environment";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

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

/** The Better Auth routes the account package serves as objects. */
const OBJECT_ROUTES = [
    "/update-user",
    "/list-sessions",
    "/revoke-session",
    "/revoke-sessions",
    "/revoke-other-sessions",
];

/** The identifier prefixes of Better Auth's models. */
const MODEL_PREFIXES: Readonly<Record<string, string>> = {
    user: "user",
    account: "user-login",
    session: "session",
    verification: "verification",
    rateLimit: "rate-limit",
    passkey: "passkey",
    factor: "factor",
    challenge: "challenge",
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
    /** The deployment's applications at the providers people sign in with. */
    providers: Readonly<Partial<Record<ProviderId, ProviderClient>>>;
    /** The account objects, which create a new person's personal account and decide access. */
    objects: Pick<
        ObjectStore,
        "mutate" | "claims" | "authorizer" | "database" | "as" | "environment"
    >;
    /** The event log the journal stores audited calls in. */
    events: EventStore;
    /** How long the packages' access tokens live. */
    lifetime: TokenLifetime;
    /** The transport mailing sign-in links and codes. */
    mail: MailTransport;
}

/** The universe's sign-in through Better Auth, answering below the issuer's URL. */
export class BetterAuthAuthenticator {
    /** The Better Auth instance storing sessions, codes and provider sign-ins. */
    readonly auth: BetterAuth;
    /** The authoritative database for sessions and account permissions. */
    readonly database: DatabaseConnection;
    /** The environment sign-in reads time at and draws identifiers from, its objects'. */
    readonly environment: Environment;
    /** The keys of the universe's identity, which sign its tokens. */
    readonly keystore: IdentityKeystore;
    /** The issuer's URL, below which every request is answered. */
    readonly issuer: string;
    /** The sign-in pages, answered before the protocol. */
    readonly #pages: SignInPage;
    /** The authorization server, whose endpoints answer before Better Auth's protocol. */
    readonly #server: AuthorizationServer;
    /** The audited authorization server. */
    readonly #authorization: Fetch;
    /** The audited Better Auth protocol. */
    readonly #protocol: Fetch;

    /** The clock sign-in reads time at, its environment's. */
    get clock(): Clock {
        return this.environment.clock;
    }

    /** Answer sign-in through a Better Auth instance, its pages and its audited protocol. */
    private constructor(
        options: BetterAuthAuthenticatorOptions,
        auth: BetterAuth,
        pages: SignInPage,
        server: AuthorizationServer,
        authorization: Fetch,
        protocol: Fetch,
    ) {
        // store the deployment's database, clock, keys and issuer
        this.database = options.database;
        this.environment = options.objects.environment;
        this.keystore = options.keystore;
        this.issuer = options.issuer;

        // answer through the instance, its pages and its protocol
        this.auth = auth;
        this.#pages = pages;
        this.#server = server;
        this.#authorization = authorization;
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
                environment: options.objects.environment,
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
        const auth = configure(options, audit, pages, secrets);

        // serve the authorization server, reading browsers' sessions without refreshing them
        const { environment } = options.objects;
        const server = new AuthorizationServer({
            issuer: options.issuer,
            database: options.database,
            objects: options.objects,
            environment,
            authorizer: options.objects.authorizer,
            lifetime: options.lifetime,
            sign: (claims, header) =>
                keystore.sign(options.database, Scope.universe.id, claims, header),
            keys: () =>
                Identity.keys(
                    Snapshot.live(options.database),
                    Scope.universe.id,
                    environment.crypto,
                ),
            session: async (headers) =>
                (await readBrowserSession(auth, headers, environment.clock))?.session.id,
            codes,
            context: auth.$context,
            pages: {
                signIn: pages.signIn,
                consent: pages.consent,
                verification: pages.verification,
            },
        });

        // serve the sign-in pages over the audited protocol and the audited authorization server
        const protocol = auditedProtocol(auth, audit, environment.clock);
        const authorization = audited(
            audit,
            server.paths,
            (headers) => readBrowserSession(auth, headers, environment.clock),
            (request) => environment.clock.track(server.answer(request)),
        );
        const signIn = new SignInPage({
            ...pages,
            endpoint: options.issuer,
            context: auth.$context,
            send: protocol,
            database: options.database,
            server,
        });

        return new BetterAuthAuthenticator(options, auth, signIn, server, authorization, protocol);
    }

    /** Read the key set the universe's tokens verify against: its identity's signing keys. */
    keys(): Promise<KeySet> {
        return Identity.keys(
            Snapshot.live(this.database),
            Scope.universe.id,
            this.environment.crypto,
        );
    }

    /** Answer a request below the issuer: its key set, a page, or an endpoint. */
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
        // answer an endpoint of the authorization server
        else if (this.#server.handles(below.pathname)) {
            return this.#authorization(placed);
        }
        // answer a protocol request
        else {
            return this.#protocol(placed);
        }
    }
}

/** The Better Auth instance of the universe's sign-in. */
type BetterAuth = ReturnType<typeof configure>;

/** Configure Better Auth with audited sessions over the account tables. */
function configure(
    options: BetterAuthAuthenticatorOptions,
    audit: BetterAuthAudit,
    pages: Pick<SignInPage, keyof typeof SIGN_IN_PATHS>,
    secrets: Awaited<ReturnType<typeof sessionSecrets>>,
) {
    return betterAuth({
        ...settings(options, audit, secrets),
        ...MODELS,
        plugins: plugins(options, audit, pages),
    });
}

/** Answer Better Auth's protocol requests through the audit as the session's user. */
function auditedProtocol(
    authentication: Pick<ReturnType<typeof betterAuth<typeof MODELS>>, "api" | "handler">,
    audit: BetterAuthAudit,
    clock: Clock,
): Fetch {
    // list the protocol's routes once
    const routes = Object.values(authentication.api)
        .map((endpoint) => endpoint.path)
        .filter((path): path is string => typeof path === "string");

    // answer requests on the clock's tracked work, as the user of the presented session
    return audited(
        audit,
        routes,
        (headers) => readBrowserSession(authentication, headers, clock),
        (request) => clock.track(authentication.handler(request)),
    );
}

/** Answer requests through the audit as the user of the session each presents. */
function audited(
    audit: BetterAuthAudit,
    routes: readonly string[],
    session: (headers: Headers) => Promise<{ readonly user: { readonly id: string } } | null>,
    handler: Fetch,
): Fetch {
    return async (request) => {
        const current = await session(request.headers);

        return audit.invoke(request, BetterAuthAudit.caller(current), routes, handler);
    };
}

/** Read the session a browser's request presents, without refreshing it. */
function readBrowserSession(
    authentication: Pick<ReturnType<typeof betterAuth<typeof MODELS>>, "api">,
    headers: Headers,
    clock: Clock,
) {
    return readSession(headers, (presented) =>
        clock.track(
            authentication.api.getSession({ headers: presented, query: { disableRefresh: true } }),
        ),
    );
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
    return {
        appName: "Destack",
        baseURL: new URL(options.issuer).origin,
        basePath: new URL(options.issuer).pathname,
        secrets,
        database: betterAuthAdapter(options.database, options.objects),
        trustedOrigins: options.trustedOrigins,
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
            database: {
                generateId: ({ model }) => generateIdentifier(model, options.objects.environment),
            },
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
) {
    return [
        passkeys(options),
        SignInProvider.plugin(options.providers, options.objects.environment),
        betterAuthSecondFactor({
            challengeUri: pages.secondFactor.href,
            keyring: options.keystore.keyring,
            environment: options.objects.environment,
        }),
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
        betterAuthSessionPolicy(options.database, options.objects.environment.clock),
        audit.plugin(),
    ];
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
    headers: Headers,
    read: (headers: Headers) => Promise<Session | null>,
): Promise<Session | null> {
    // read no session without credentials
    if (!headers.has("cookie") && !headers.has("authorization")) {
        return null;
    }

    // treat a revoked credential as no actor
    try {
        return await read(headers);
    } catch (error) {
        // rethrow any other failure
        if (!isAPIError(error) || error.statusCode !== 401) {
            throw error;
        }

        return null;
    }
}

/** Generate the identifier required by the shared authentication model. */
function generateIdentifier(model: string, environment: UuidEnvironment): string {
    // refuse an unknown model
    const prefix = Object.hasOwn(MODEL_PREFIXES, model) ? MODEL_PREFIXES[model] : undefined;
    if (prefix === undefined) {
        throw AccountError.of(
            "AUTHENTICATION_MODEL_UNKNOWN",
            `unknown authentication model: ${model}`,
        );
    }

    return Identifier.create(prefix, environment);
}

/** Mail a sign-in link or code to an address, refusing a message the transport did not send. */
async function mail(
    options: Pick<BetterAuthAuthenticatorOptions, "mail" | "objects">,
    address: string,
    content: MailContent,
): Promise<void> {
    // send the content to the address under a fresh key
    const outcome = await options.mail.send({
        ...content,
        to: address,
        key: Uuid.v4(options.objects.environment.random),
        date: options.objects.environment.clock.now(),
    });

    // refuse a message the transport retried or failed
    if (outcome.kind !== "sent") {
        throw AccountError.of(
            "MAIL_UNSENT",
            `sign-in mail to ${address} was not sent: ${outcome.refusal.message}`,
        );
    }
}

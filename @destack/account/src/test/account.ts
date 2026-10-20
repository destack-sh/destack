import {
    and,
    eq,
    type Database,
    type DatabaseConnection,
    type Dialect,
    Snapshot,
} from "@destack/db";
import { type ImageIndex, TEST_DIALECTS, TestDatabase, TestTemplate } from "@destack/db/test";
import {
    Replica,
    replicaPosition,
    Scope,
    type ObjectReference,
    type Subject,
    type Subscription,
} from "@destack/sync";
import { RequestId } from "@destack/runtime/request";
import {
    AccessContext,
    anyone,
    type Authentication,
    Caller,
    type PermissionReference,
    type Policy,
    principal,
    Relationship,
} from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { type Keychain, type Keyring, LocalKeyring, MemoryKeychain } from "@destack/identity";
import { claim } from "../object/claim.ts";
import { placement } from "../object/placement.ts";
import objectServer, { ObjectStore } from "@destack/object/server";
import { testEvents } from "@destack/object/test";
import { Package, PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { ResourceContext } from "@destack/resource/context";
import {
    canonicalize,
    Digest,
    found,
    Identifier,
    type JsonValue,
    present,
    schema,
} from "@destack/schema";
import { PackageMount } from "@destack/runtime";
import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/runtime/authentication";
import { CallError } from "@destack/runtime/error";
import { Health } from "@destack/runtime/health";
import { Server, type PackageImplementation } from "@destack/runtime/server";
import { testCallKey } from "@destack/runtime/test";
import { createClient, type ClientOf, type ClientTarget } from "@destack/runtime/client";
import type { ProcedureRouter } from "@destack/runtime";
import { Setting } from "@destack/setting";
import { settingServer } from "@destack/setting/server";
import {
    AccountAuthentication,
    BetterAuthAuthenticator,
    type BetterAuthAuthenticatorOptions,
    type SignUp,
} from "../better-auth/index.ts";
import { MachineIdentity } from "../client/index.ts";
import { IdentityKeystore, IdentityRegistry } from "../identity/index.ts";
import {
    account,
    Account,
    client,
    machine,
    ProviderCode,
    Residency,
    residency,
    type ResidencyCode,
    type ResidencyDefinition,
    session,
    SpaceKind,
    StackSource,
    user,
    User,
} from "../object/index.ts";
import { JWT_BEARER, SESSION_TOKEN_TYPE, TOKEN_EXCHANGE } from "../oauth/index.ts";
import accountPackage, { OAUTH_PATH } from "../package.ts";
import { accountServer, Operator, OperatorDefinition, UniverseRole } from "../server/index.ts";
import { accountDatabase } from "../stack/index.ts";
import { TOKEN_PATH } from "../authentication/path.ts";
import { TokenResponse } from "../authentication/token.ts";
import { Browser } from "./browser.ts";
import { TestEntitlement } from "./entitlement.ts";
import type { Mail, MailOutcome, MailTransport } from "../better-auth/index.ts";
import { Clock } from "@destack/environment";

/** The universe's origin. */
export const ORIGIN = "http://localhost:3210";

/** The URL the fixture's users, machines and installations reach the account package at. */
export const ACCOUNTS_URL = PackageMount.url(ORIGIN, accountPackage.id);

/** The universe's issuer: the account package's oauth adapter. */
export const ISSUER = `${ACCOUNTS_URL}${OAUTH_PATH}`;

/** The email of the person operating the fixture's universe. */
export const OPERATOR_EMAIL = "operator@example.com";

/** The time zone and locale the fixture's people sign up with. */
export const SIGN_UP: Pick<SignUp, "timeZone" | "locale"> = {
    timeZone: "Europe/Vienna",
    locale: "en",
};

/** The handle of the platform's own account, whose home serves its residency. */
export const PLATFORM_HANDLE = "destack";

/** The person owning the fixture's accounts. */
const OWNER = schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000a");

/** The identities the account package stores from its start. */
export const ids = {
    owner: OWNER,
    stranger: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000b"),
    operator: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000c"),
    account: Account.personal(OWNER),
    other: schema.identifier("account").parse("account-01996ab0-0000-7000-8000-000000000002"),
    machine: schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-000000000004"),
    usMachine: schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-000000000006"),
};

/** Who serves the residencies' homes: the fixture as their cloud machines, or a real machine. */
export type HomeServer = "fixture" | "machine";

/** The stand-in stacks spaces are born with. */
export const STACKS: Readonly<Record<SpaceKind, StackSource>> = {
    residency: {
        package: Package.parse({
            id: "package-01996ab0-0000-7000-8000-0000000000b1",
            name: "@example/residency",
            version: "2026.9.0",
        }),
        manifest: Digest.parse("a".repeat(64)),
        export: "RESIDENCY",
        settings: {},
    },
    person: {
        package: Package.parse({
            id: "package-01996ab0-0000-7000-8000-0000000000b2",
            name: "@example/home",
            version: "2026.9.0",
        }),
        manifest: Digest.parse("b".repeat(64)),
        export: "HOME",
        settings: {},
    },
    space: {
        package: Package.parse({
            id: "package-01996ab0-0000-7000-8000-0000000000b3",
            name: "@example/space",
            version: "2026.9.0",
        }),
        manifest: Digest.parse("d".repeat(64)),
        export: "SPACE",
        settings: {},
    },
};

/** The header with the user, space or installation a test request acts as, instead of its proof. */
const CALLER_HEADER = "x-caller";

/** How long a platform installation's replicas take at most to catch up with accounts. */
const SETTLE_MILLISECONDS = 5000;

/** How long a stopping fixture server drains its requests, in milliseconds. */
const DRAIN_MILLISECONDS = 1000;

/** How long before its expiry the fixture exchanges a person's token again, in milliseconds. */
const TOKEN_REFRESH_MILLISECONDS = 10_000;

/** The provider running the platform's cloud machines. */
const PROVIDER = ProviderCode.parse("cloudflare");

/** The provider location of the platform's cloud machines. */
const LOCATION = "eu";

/** The operating system and hardware of the personal machines the fixture enrolls. */
const PERSONAL_MACHINE = {
    kind: "personal",
    platform: "macOS",
    architecture: "arm64",
    model: "Mac14,2",
} as const;

/** The creation and update times of the rows the fixture stores. */
const RECORD_TIMES = { createdAt: 1, updatedAt: 1 };

/** The residencies of the universe, as the platform's bootstrap declares them. */
const RESIDENCIES: readonly ResidencyDefinition[] = [
    { code: "eu", name: "European Union" },
    { code: "us", name: "United States" },
];

/** The people the account package stores from its start, with their names and emails. */
const PEOPLE = [
    { id: ids.owner, name: "Owner", email: `${ids.owner}@example.test` },
    { id: ids.stranger, name: "Stranger", email: `${ids.stranger}@example.test` },
    { id: ids.operator, name: "Operator", email: OPERATOR_EMAIL },
] as const;

/** The owner's accounts the account package stores from its start, with their handles and kinds. */
const ACCOUNTS = [
    { id: ids.account, handle: "acme", kind: "personal" },
    { id: ids.other, handle: "rival", kind: "shared" },
] as const;

/** A token endpoint's refusal (RFC 6749 5.2). */
const Refusal = schema.looseObject({
    /** The error code. */
    error: schema.string(),
    /** What went wrong. */
    error_description: schema.string(),
});

/** How a test serves the account package. */
export interface AccountFixtureOptions {
    /** The database dialect, the last test dialect by default. */
    readonly dialect?: Dialect;
    /** Open the release declaring a setting value. */
    readonly openRelease?: (packageId: PackageId, version: string) => Promise<BuildReader>;
    /** The policies of other packages' types. */
    readonly policies?: readonly Policy[];
    /** The sign-in options the test changes. */
    readonly signIn?: Partial<BetterAuthAuthenticatorOptions>;
    /** The template images to copy from, the run's when absent. */
    readonly images?: ImageIndex;
}

/** How a test serves the account package over its template's stacks and homes. */
type ServeOptions = AccountFixtureOptions & {
    /** The stacks spaces are born with. */
    readonly stacks: Readonly<Record<SpaceKind, StackSource>>;
    /** Who serves the residencies' homes. */
    readonly homes: HomeServer;
};

/** The rows an account template writes, and the world a further write puts on them. */
export interface AccountTemplateOptions<World extends JsonValue> {
    /** The template's name in its run. */
    readonly name: string;
    /** The stacks spaces are born with, STACKS by default. */
    readonly stacks?: Readonly<Record<SpaceKind, StackSource>>;
    /** Who serves the residencies' homes, the fixture by default. */
    readonly homes?: HomeServer;
    /** The dialects tests copy it in, every test dialect by default. */
    readonly dialects?: readonly Dialect[];
    /** How the template's write serves the account package. */
    readonly serve?: AccountFixtureOptions;
    /** Write a world on the fixture's rows, returning what its copies share. */
    readonly world: {
        /** What the world's copies share, as its JSON form crosses to the tests. */
        readonly value: schema.Schema<World>;
        /** Write the world over the served fixture, keeping files in a run's directory. */
        readonly write: (accounts: AccountFixture, directory: string) => Promise<World>;
    };
}

/** What an account template's copies share: its write's keys, stacks and homes, and its world. */
export type AccountTemplateValue<World extends JsonValue> = {
    /** The keychains' secrets the write ended with. */
    readonly keys: AccountKeysValue;
    /** The stacks spaces are born with. */
    readonly stacks: Readonly<Record<SpaceKind, StackSource>>;
    /** Who serves the residencies' homes. */
    readonly homes: HomeServer;
    /** What the world's write returned. */
    readonly world: World;
};

/** The universe's store, sign-in and servers, as one fixture serves them. */
interface Served {
    /** The universe's object store. */
    readonly objects: ObjectStore;
    /** The universe's sign-in. */
    readonly authenticator: BetterAuthAuthenticator;
    /** Open another sign-in over the same store and inbox with a keyring. */
    readonly authenticate: (keyring: Keyring) => Promise<BetterAuthAuthenticator>;
    /** The servers of the universe's packages, by package. */
    readonly servers: ReadonlyMap<PackageId, Server>;
    /** The fixture-verified callers, by identity. */
    readonly callers: Map<string, Verification>;
    /** The published releases declaring settings, by package and version. */
    readonly releases: Map<string, Promise<BuildReader>>;
    /** The email sign-in mailed. */
    readonly inbox: Inbox;
}

/** The account package in one database, served as the universe serves it. */
export class AccountFixture implements AsyncDisposable {
    /** The account package's test database. */
    readonly storage: TestDatabase;
    /** The account database. */
    readonly database: DatabaseConnection;
    /** The sign-in over the database. */
    readonly authenticator: BetterAuthAuthenticator;
    /** The servers of the account and setting packages, by package. */
    readonly servers: ReadonlyMap<PackageId, Server>;
    /** The universe's object store. */
    readonly objects: ObjectStore;
    /** The keys of the universe and of the residencies' home spaces, signing their tokens. */
    readonly keystore: IdentityKeystore;
    /** The users, spaces and installations tests act as, verified by the fixture, by identity. */
    readonly callers: Map<string, Verification>;
    /** The published releases declaring settings, by package and version. */
    readonly releases: Map<string, Promise<BuildReader>>;
    /** The platform's own account, whose home serves its residency. */
    readonly platform: Identifier<"account">;
    /** Who serves the residencies' homes. */
    readonly homes: HomeServer;
    /** The email sign-in mailed. */
    readonly #inbox: Inbox;
    /** The keychains storing the keys of the declared cloud machines, by machine. */
    readonly #cloudKeychains: ReadonlyMap<Identifier<"machine">, Keychain>;
    /** The keychain of the universe's keyring. */
    readonly #keychain: MemoryKeychain;
    /** Open another sign-in over the same store and inbox with a keyring. */
    readonly #authenticate: (keyring: Keyring) => Promise<BetterAuthAuthenticator>;
    /** The access tokens exchanged for sessions, by session token, audience and space. */
    readonly #tokens = new Map<string, Promise<{ accessToken: string; expiresAt: number }>>();

    /** Store a served account package. */
    private constructor(
        storage: TestDatabase,
        served: Served,
        state: {
            /** The platform's own account. */
            readonly platform: Identifier<"account">;
            /** Who serves the residencies' homes. */
            readonly homes: HomeServer;
            /** The keychains storing the keys of the declared cloud machines, by machine. */
            readonly cloudKeychains: ReadonlyMap<Identifier<"machine">, Keychain>;
            /** The keychain of the universe's keyring. */
            readonly keychain: MemoryKeychain;
        },
    ) {
        // store the served store, sign-in and servers over the database
        this.storage = storage;
        this.database = storage.database;
        this.objects = served.objects;
        this.authenticator = served.authenticator;
        this.servers = served.servers;
        this.keystore = served.authenticator.keystore;

        // store what tests read and change
        this.callers = served.callers;
        this.releases = served.releases;
        this.platform = state.platform;
        this.homes = state.homes;
        this.#inbox = served.inbox;
        this.#cloudKeychains = state.cloudKeychains;
        this.#keychain = state.keychain;
        this.#authenticate = served.authenticate;
    }

    /** Define a template of the fixture's rows, and of a world a further write puts on them. */
    static template<World extends JsonValue>(
        options: AccountTemplateOptions<World>,
    ): TestTemplate<AccountTemplateValue<World>> {
        // write the fixture's rows with the template's stacks and homes
        const stacks = options.stacks ?? STACKS;
        const homes = options.homes ?? "fixture";
        const world = options.world;

        return new TestTemplate({
            name: options.name,
            tables: accountDatabase,
            ...(options.dialects === undefined ? {} : { dialects: options.dialects }),
            value: schema.object({
                keys: AccountKeys.schema,
                stacks: schema.record(SpaceKind, StackSource),
                homes: schema.enum(["fixture", "machine"]),
                world: world.value,
            }),
            write: async (storage, directory) => {
                // write and serve the fixture's rows, then the world over them
                const served = { ...options.serve, stacks, homes };
                const keys = await AccountFixture.#write(storage, served);
                const accounts = await AccountFixture.#over(
                    storage,
                    await AccountFixture.#serve(storage, keys, served),
                    keys,
                    homes,
                );
                const value = await world.write(accounts, directory);
                await accounts.close();

                return { keys: keys.value(), stacks, homes, world: value };
            },
        });
    }

    /** Write and serve the fixture's rows in a fresh database, as a simulation builds its own. */
    static async create(
        options: AccountFixtureOptions &
            Partial<Pick<AccountTemplateOptions<null>, "stacks" | "homes">> = {},
    ): Promise<AccountFixture> {
        // write the fixture's rows with the given stacks and homes, the defaults when absent
        const storage = await TestDatabase.create(
            options.dialect ?? present(TEST_DIALECTS.at(-1), "the last test dialect"),
            accountDatabase,
            { isMigrated: true },
        );
        const { stacks = STACKS, homes = "fixture", ...rest } = options;
        const served = { ...rest, stacks, homes };
        const keys = await AccountFixture.#write(storage, served);

        // serve them
        return await AccountFixture.#over(
            storage,
            await AccountFixture.#serve(storage, keys, served),
            keys,
            served.homes,
        );
    }

    /** Serve the account package with the owner's accounts and the platform's organisation. */
    static async open(options: AccountFixtureOptions = {}): Promise<AccountFixture> {
        return (await AccountFixture.copy(ACCOUNT_TEMPLATE, options)).accounts;
    }

    /** Serve a copy of a template's rows, with the world its write put on them. */
    static async copy<World extends JsonValue>(
        template: TestTemplate<AccountTemplateValue<World>>,
        options: AccountFixtureOptions = {},
    ): Promise<{ readonly accounts: AccountFixture; readonly world: World }> {
        // copy the template, confirming its replicas as of now
        const { storage, value } = await TestDatabase.copy(
            options.dialect ?? present(TEST_DIALECTS.at(-1), "the last test dialect"),
            template,
            options.images,
        );
        await storage.database.update(replicaPosition).set({ confirmedAt: Date.now() });

        // serve it with the keys, stacks and homes the template's write ended with
        const keys = AccountKeys.of(value.keys);
        const { stacks, homes } = value;
        const served = await AccountFixture.#serve(storage, keys, { ...options, stacks, homes });

        return {
            accounts: await AccountFixture.#over(storage, served, keys, homes),
            world: value.world,
        };
    }

    /** Open a migrated account database with the universe's residencies. */
    static async storage(
        definition: Database = accountDatabase,
        dialect: Dialect = present(TEST_DIALECTS.at(-1), "the last test dialect"),
    ): Promise<TestDatabase> {
        const storage = await TestDatabase.create(dialect, definition, { isMigrated: true });
        await AccountFixture.#insertResidencies(storage.database);

        return storage;
    }

    /** Read a residency's home space, the home of the platform's account in it. */
    async home(residencyId: ResidencyCode): Promise<Identifier<"space">> {
        const snapshot = Snapshot.live(this.database);

        return Residency.home(
            { get: (input) => Residency.require(snapshot, input.id) },
            residencyId,
        );
    }

    /** Route a request of the universe's origin to the package mounted at its path. */
    fetch(request: Request): Promise<Response> {
        // find the mounted package, refusing a path of none served
        const mount = PackageMount.of(request);
        const packageId = PackageId.safeParse(mount?.packageId);
        const server = packageId.success ? this.servers.get(packageId.data) : undefined;
        if (mount === undefined || server === undefined) {
            return Promise.resolve(new Response(null, { status: 404 }));
        }

        return server.fetch(mount.request);
    }

    /** Store a verified person with their eu personal account, a session and a client. */
    async person(email: string, handle: string = freshHandle()): Promise<Person> {
        // store the user and their personal account as the sign-up creates them
        const context = await this.authenticator.auth.$context;
        const created = await context.internalAdapter.createUser(
            { email, name: email, emailVerified: true, ...SIGN_UP, handle, residencyId: "eu" },
            { method: "magic-link" },
        );
        const userId = schema.identifier("user").parse(created.id);
        const token = await this.signIn(userId);

        return {
            id: userId,
            subject: principal.user.reference(Scope.universe.id, userId),
            ...(await this.#accountOf(userId)),
            session: token,
            client: this.sessionClient(token, accountPackage),
        };
    }

    /** Sign a stored user in on a new client as an email sign-in does, returning the session. */
    async signIn(userId: Identifier<"user">): Promise<string> {
        // create the session and record the email sign-in as its ceremony
        const context = await this.authenticator.auth.$context;
        const created = await context.internalAdapter.createSession(userId);
        await context.internalAdapter.updateSession(created.token, {
            authenticatedAt: this.objects.clock.now(),
            authenticationMethod: "magic-link",
        });

        // register the client the person signed in on, and attach the session to it
        const clientId = Identifier.create("client");
        const now = this.objects.clock.now();
        await this.database.insert(client.table).values({
            id: clientId,
            scope: userId,
            name: "Test client",
            kind: "native",
            createdAt: now,
            updatedAt: now,
        });
        await this.database
            .update(session.table)
            .set({ clientId })
            .where(eq(session.table.token, created.token));

        return created.token;
    }

    /** Connect a typed client acting as a stored user through a new session's access tokens. */
    async session(userId: Identifier<"user">): Promise<ClientOf<typeof accountPackage>> {
        return this.sessionClient(await this.signIn(userId), accountPackage);
    }

    /** Connect a typed client of a universe package acting through a session's tokens. */
    sessionClient<Handle extends ClientTarget<ProcedureRouter> & { readonly id: PackageId }>(
        sessionToken: string,
        handle: Handle,
    ): ClientOf<Handle> {
        return createClient<Handle["router"]>(handle, {
            url: PackageMount.url(ORIGIN, handle.id),
            fetch: async (request: Request) => {
                // replace the request's credential with the session's access token
                const { accessToken } = await this.exchange(sessionToken, handle.id);
                const headers = new Headers(request.headers);
                headers.set("authorization", `Bearer ${accessToken}`);

                return this.fetch(new Request(request, { headers }));
            },
        });
    }

    /** Exchange a session for a package's token, in a space when given, reusing a fresh one. */
    exchange(
        sessionToken: string,
        audience: PackageId,
        spaceId?: string,
    ): Promise<{ accessToken: string; expiresAt: number }> {
        // reuse a fresh token
        const key = JSON.stringify([sessionToken, audience, spaceId ?? null]);
        const cached = this.#tokens.get(key);
        const exchanging = (async () => {
            // answer the cached token while it lives
            const current = await cached;
            if (
                current !== undefined &&
                current.expiresAt - TOKEN_REFRESH_MILLISECONDS > this.objects.clock.now()
            ) {
                return current;
            }

            // exchange the session at the token endpoint
            return this.exchangeToken(sessionToken, SESSION_TOKEN_TYPE, {
                audience,
                ...(spaceId === undefined ? {} : { space: spaceId }),
            });
        })();
        this.#tokens.set(key, exchanging);
        exchanging.catch(() => this.#tokens.delete(key));

        return exchanging;
    }

    /** Exchange a subject token at the token endpoint (RFC 8693), refusing as it refuses. */
    async exchangeToken(
        subjectToken: string,
        type: string,
        asked: {
            /** The package the token is for. */
            readonly audience: PackageId;
            /** The space it acts in. */
            readonly space?: string;
            /** The person to act as or for, as a subject key. */
            readonly subject?: string;
            /** The chain of grants to act under, space-delimited. */
            readonly grants?: string;
            /** The person's session the token lives within. */
            readonly session?: string;
        },
    ): Promise<{ accessToken: string; expiresAt: number }> {
        // post the subject token for the package's resource
        const { audience, ...parameters } = asked;
        const response = await this.fetch(
            new Request(`${ISSUER}${TOKEN_PATH}`, {
                method: "POST",
                body: new URLSearchParams({
                    grant_type: TOKEN_EXCHANGE,
                    subject_token: subjectToken,
                    subject_token_type: type,
                    resource: PackageId.resource(audience),
                    ...parameters,
                }),
            }),
        );

        // refuse as the endpoint refused, by its error and description
        if (!response.ok) {
            const refused = Refusal.parse(await response.json());
            throw new CallError("BAD_REQUEST", {
                message: `${refused.error}: ${refused.error_description}`,
            });
        }

        return TokenResponse.granted(TokenResponse.parse(await response.json()));
    }

    /** Start a space's identity as the machine serving it, its keys in the fixture's keystore. */
    async startIdentity(spaceId: Identifier<"space">, serving: Actor): Promise<void> {
        const registry = IdentityRegistry.client(serving.client, this.objects.clock);
        const keystore = new IdentityKeystore(this.keystore.keyring, registry, this.objects.clock);
        await keystore.generate(this.database, spaceId);
    }

    /** Exchange an installation's space-signed assertion for a package's token in it (RFC 7523). */
    async installationToken(
        installation: Subject,
        audience: PackageId = accountPackage.id,
    ): Promise<string> {
        // post the assertion for the package's resource
        const endpoint = `${ISSUER}${TOKEN_PATH}`;
        const response = await this.fetch(
            new Request(endpoint, {
                method: "POST",
                body: new URLSearchParams({
                    grant_type: JWT_BEARER,
                    assertion: await this.keystore
                        .source(this.database, installation, {
                            issuer: ISSUER,
                            send: (request) => this.fetch(request),
                        })
                        .assertion(endpoint),
                    resource: PackageId.resource(audience),
                    space: installation.scope,
                }),
            }),
        );

        // refuse as the endpoint refused
        if (!response.ok) {
            const refused = Refusal.parse(await response.json());
            throw new CallError("BAD_REQUEST", {
                message: `${refused.error}: ${refused.error_description}`,
            });
        }

        return TokenResponse.parse(await response.json()).access_token;
    }

    /** Sign a person up with an eu personal account through a mailed link in a new browser. */
    async signInWithLink(
        email: string,
        handle: string = freshHandle(),
    ): Promise<Person & { readonly browser: Browser }> {
        // follow the mailed link
        const browser = this.browser();
        await browser.fetch("/sign-in/magic-link", {
            email,
            callbackURL: "/",
            metadata: { ...SIGN_UP, name: email, handle, residencyId: "eu" },
        });
        const links = await this.links();
        await browser.fetch(present(links.at(-1), "the mailed link").url);

        // read the signed-in user through the browser's session
        const current = schema
            .looseObject({ user: schema.looseObject({ id: schema.identifier("user") }) })
            .parse(await (await browser.fetch("/get-session")).json());
        const userId = current.user.id;

        // sign the person in on a client too
        const token = await this.signIn(userId);

        return {
            browser,
            session: token,
            client: this.sessionClient(token, accountPackage),
            id: userId,
            subject: principal.user.reference(Scope.universe.id, userId),
            ...(await this.#accountOf(userId)),
        };
    }

    /** Record a second factor as the last ceremony of a user's sessions. */
    async elevate(userId: Identifier<"user">): Promise<void> {
        await this.database
            .update(session.table)
            .set({ authenticationMethod: "totp", authenticatedAt: this.objects.clock.now() })
            .where(eq(session.table.userId, userId));
        this.#tokens.clear();
    }

    /** Open another sign-in over the database, the universe's root key rotated. */
    async rotate(): Promise<BetterAuthAuthenticator> {
        return this.#authenticate(await LocalKeyring.rotate(this.#keychain, "account"));
    }

    /** Sign a stored user's access token for a package in a space through a new session. */
    async token(userId: Identifier<"user">, audience: PackageId, spaceId: string): Promise<string> {
        return (await this.exchange(await this.signIn(userId), audience, spaceId)).accessToken;
    }

    /** Read the sign-in links mailed so far, oldest first. */
    links(): ReturnType<Inbox["links"]> {
        return this.#inbox.links();
    }

    /** Read the one-use codes mailed so far with what each is for, oldest first. */
    codes(): ReturnType<Inbox["codes"]> {
        return this.#inbox.codes();
    }

    /** Read the last one-use code the issuer mailed. */
    async code(): Promise<string> {
        const codes = await this.#inbox.codes();

        return present(codes.at(-1), "a mailed code").otp;
    }

    /** Open a browser of the issuer's pages and protocol. */
    browser(language: string = SIGN_UP.locale): Browser {
        return new Browser(ISSUER, (request) => this.fetch(request), language);
    }

    /** Place a new space in a new shared account of a person, on a machine enrolled in it. */
    async createSpace(person: Person, residencyId: ResidencyCode = "eu") {
        const created = await person.client.account.create({
            kind: "shared",
            scope: person.id,
            requestId: RequestId.create(this.objects.clock.now()),
            handle: freshHandle(),
            name: "Account",
            residencyId,
        });

        return this.#place(person, created, Identifier.create("space"));
    }

    /** Read a person's home, placed at sign-up, with the cloud machine serving it as a caller. */
    async personalHome(person: Person) {
        const spaceId = Account.home(person.account);

        return {
            accountId: person.account,
            spaceId,
            machine: this.caller(await this.serving(spaceId)),
        };
    }

    /** Act as the cloud machine a space is placed on. */
    async serving(spaceId: Identifier<"space">): Promise<MachineIdentity> {
        const placed = present(
            await Snapshot.live(this.database)
                .read(placement.table)
                .findFirst({ where: { id: spaceId } }),
            `the placement of ${spaceId}`,
        );
        const machineId = placed.machineId;

        return new MachineIdentity(machineId, this.cloudKeychain(machineId), this.objects.clock);
    }

    /** Enroll a new machine of an account through the machine API, and return its identity. */
    async enroll(
        accountId: Identifier<"account">,
        options: MachineEnrollment = {},
    ): Promise<MachineIdentity> {
        // enroll a new identity as the client given, else the account's owner or the platform's operator
        const identity = new MachineIdentity(
            Identifier.create("machine"),
            new MemoryKeychain(),
            this.objects.clock,
        );
        const enroller = accountId === this.platform ? ids.operator : ids.owner;
        await identity.enroll(options.client ?? this.user(enroller, accountPackage), {
            scope: accountId,
            requestId: RequestId.create(this.objects.clock.now()),
            name: options.name ?? `machine-${identity.machineId.slice(-12)}`,
            ...(options.residencyId === undefined
                ? PERSONAL_MACHINE
                : {
                      kind: "cloud",
                      residencyId: options.residencyId,
                      providerCode: PROVIDER,
                      location: LOCATION,
                  }),
        });

        return identity;
    }

    /** Declare a cloud machine of an account serving a residency, and answer its identity. */
    async declareCloudMachine(
        accountId: Identifier<"account">,
        residencyId: ResidencyCode,
    ): Promise<MachineIdentity> {
        // store a new identity's machine as the universe with its key
        const identity = new MachineIdentity(
            Identifier.create("machine"),
            new MemoryKeychain(),
            this.objects.clock,
        );
        const { publicKey } = await identity.generate();
        await this.objects.mutate(this.objects.serving(machine), {
            id: RequestId.create(this.objects.clock.now()),
            calls: [
                machine.call("declare", {
                    ...machine.target({ scope: accountId, id: identity.machineId }),
                    name: `cloud-${identity.machineId.slice(-12)}`,
                    kind: "cloud",
                    residencyId,
                    providerCode: PROVIDER,
                    location: LOCATION,
                    publicKey,
                }),
            ],
        });

        return identity;
    }

    /** Connect to the account package as a machine, with the tokens its key proves. */
    machine(identity: MachineIdentity): ClientOf<typeof accountPackage> {
        return createClient(accountPackage, {
            url: ACCOUNTS_URL,
            fetch: identity.fetch((request) => this.fetch(request), accountPackage.id, {
                url: ISSUER,
            }),
        });
    }

    /** Connect a caller acting as a machine, with the tokens its key proves. */
    caller(identity: MachineIdentity): Actor {
        return {
            id: identity.machineId,
            subject: principal.machine.reference(Scope.universe.id, identity.machineId),
            client: this.machine(identity),
        };
    }

    /** Connect to a universe package as a stored user, verified by the fixture. */
    user<Handle extends ClientTarget<ProcedureRouter> & { readonly id: PackageId }>(
        userId: string,
        handle: Handle,
    ): ClientOf<Handle> {
        const subject = principal.user.reference(Scope.universe.id, userId);
        this.callers.set(userId, { kind: "fixture", id: userId, subject });

        return this.connect(() => ({ [CALLER_HEADER]: userId }), handle);
    }

    /** Connect a typed client with an access token. */
    bearer(token: string): ClientOf<typeof accountPackage> {
        return this.connect(() => ({ authorization: `Bearer ${token}` }), accountPackage);
    }

    /** Install a platform installation in a residency's home space. */
    async install(
        permissions: readonly PermissionReference[] = [],
        residencyId: ResidencyCode = "eu",
    ): Promise<Subject> {
        // take a new installation of the home space, which serves the residency as every installation there does
        const installationId = Identifier.create("installation");
        const installation = principal.installation.reference(
            await this.home(residencyId),
            installationId,
        );

        // bind it a universe role with the permissions
        if (permissions.length > 0) {
            await new AccessFixture(this.database).replicateRole(
                Scope.universe,
                {
                    name: `installation-${installationId.slice(-12)}`,
                    description: "Serves a residency",
                    permissions: [...permissions],
                },
                installation,
            );
        }

        return installation;
    }

    /** Connect to the account package as an installation, verified by the fixture in its space. */
    installation(installation: Subject): ClientOf<typeof accountPackage> {
        this.callers.set(installation.id, {
            kind: "space-key",
            id: installation.scope,
            subject: installation,
        });

        return this.connect(() => ({ [CALLER_HEADER]: installation.id }), accountPackage);
    }

    /** Connect a caller that acts as a space or one of its installations. */
    space(spaceId: string, installationId?: string): Actor {
        // verify the caller as the space or its installation alone
        const subject =
            installationId === undefined
                ? principal.space.reference(Scope.universe.id, spaceId)
                : principal.installation.reference(spaceId, installationId);
        this.callers.set(subject.id, { kind: "space-key", id: spaceId, subject });

        return {
            id: subject.id,
            subject,
            client: this.connect(() => ({ [CALLER_HEADER]: subject.id }), accountPackage),
        };
    }

    /** Connect a typed client of a universe package sending the given headers. */
    connect<Handle extends ClientTarget<ProcedureRouter> & { readonly id: PackageId }>(
        headers: () => Record<string, string>,
        handle: Handle,
    ): ClientOf<Handle> {
        return createClient<Handle["router"]>(handle, {
            url: PackageMount.url(ORIGIN, handle.id),
            headers,
            fetch: (request) => this.fetch(request),
        });
    }

    /** Wait until an installation's replicas from the account package catch up, or fail. */
    async settle(objects: ObjectStore, failure?: AbortSignal): Promise<void> {
        // wait for each replica the installation requests from the account package, while it requests new ones
        const timeout = AbortSignal.timeout(SETTLE_MILLISECONDS);
        const signal = failure === undefined ? timeout : AbortSignal.any([timeout, failure]);
        const head = await this.database.log.position();
        const done = new Set<string>();
        for (let isGrowing = true; isGrowing;) {
            // follow the replicas of the account package the store requests now, as it follows them
            const requests = await AccountFixture.#requests(objects);
            const pending = requests.filter((request) => !done.has(canonicalize(request)));

            // wait for each to catch up and to complete a run of the request it has then
            for (const request of pending) {
                const isCaught = await objects.wait({ scope: request.scope, ...head }, signal);
                const isCurrent = await objects.database.log.until(async () => {
                    const latest = (await AccountFixture.#requests(objects)).find(
                        (each) => each.name === request.name && each.scope === request.scope,
                    );

                    return (
                        latest === undefined ||
                        (await Replica.confirmedAt(objects.database, latest)) !== undefined
                    );
                }, signal);

                // fail with the failure aborting the wait, or on a replica left behind
                failure?.throwIfAborted();
                if (!isCaught || !isCurrent) {
                    throw new Error(
                        `the replica ${request.name} did not catch up with the account package`,
                    );
                }
                done.add(canonicalize(request));
            }
            isGrowing = pending.length > 0;
        }
    }

    /** Relate a subject to an object by a relation or a role, as a grant in a scope. */
    static async relate(
        database: DatabaseConnection,
        scope: string,
        object: ObjectReference,
        binding: string | { readonly role: string },
        subject: Subject,
    ): Promise<void> {
        const relationship = {
            id: Identifier.create("relationship"),
            object,
            ...(typeof binding === "string" ? { relation: binding } : binding),
            subject,
            createdAt: 1,
            expiresAt: null,
        };
        await Relationship.insert(database, relationship, scope);
    }

    /** Read the keychain storing a declared cloud machine's key. */
    cloudKeychain(machineId: Identifier<"machine">): Keychain {
        return found(this.#cloudKeychains, machineId);
    }

    /** Serve a package to the callers the fixture verifies and to bearers of access tokens. */
    static serve(
        implementation: PackageImplementation,
        database: DatabaseConnection,
        callers: ReadonlyMap<string, Verification>,
        clock: Clock,
    ): Server {
        const audience = implementation.package.id;

        return Server.start({
            ...implementation,
            clock,
            resources: new ResourceContext(),
            health: new Health(implementation.package.name),
            drainTimeout: DRAIN_MILLISECONDS,
            authorizeMachine: async () => {},
            authenticate: async (request) => {
                // act as a caller the fixture verifies now, toward the served package
                const callerName = request.headers.get(CALLER_HEADER);
                if (callerName !== null) {
                    return verified(found(callers, callerName), audience, clock.now());
                }

                // verify an access token by the universe's keys, else call anonymously
                return AccountAuthentication.bearer(request, {
                    database,
                    issuer: ISSUER,
                    audience,
                    now: clock.now(),
                });
            },
        });
    }

    /** Stop serving the packages. */
    async close(): Promise<void> {
        for (const server of this.servers.values()) {
            await server.close();
        }
    }

    /** Keep a fixture over a storage the account package serves with some keys. */
    static async #over(
        storage: TestDatabase,
        served: Served,
        keys: AccountKeys,
        homes: HomeServer,
    ): Promise<AccountFixture> {
        return new AccountFixture(storage, served, {
            platform: await AccountFixture.#platform(storage.database),
            homes,
            cloudKeychains: keys.cloud,
            keychain: keys.universe,
        });
    }

    /** Store the residencies, the people, the platform, its homes and the owner's accounts. */
    static async #write(storage: TestDatabase, options: ServeOptions): Promise<AccountKeys> {
        // store the residencies and the people
        await AccountFixture.#insertResidencies(storage.database);
        await AccountFixture.#insertPeople(storage.database);

        // serve the account package with new keys, storing the platform before the universe's identity
        const keys = AccountKeys.create();
        const definition = await AccountFixture.#platformDefinition(keys.cloud);
        const served = await AccountFixture.#serve(storage, keys, options, definition);
        const fixture = await AccountFixture.#over(storage, served, keys, options.homes);

        // seat the operator, start the homes' identities and store the owner's accounts, then stop serving
        await fixture.#seatOperator();
        if (options.homes === "fixture") {
            await fixture.#serveHomes();
        }
        await fixture.#createAccounts();
        await fixture.close();

        return keys;
    }

    /** Store each residency of the universe, shown to anyone. */
    static async #insertResidencies(database: DatabaseConnection): Promise<void> {
        const access = new AccessFixture(database);
        for (const declared of RESIDENCIES) {
            await database
                .insert(residency.table)
                .values({ ...RECORD_TIMES, id: declared.code, name: declared.name });
            await access.replicateRelationship(
                Scope.universe.id,
                residency.reference(Scope.universe.id, declared.code),
                "public",
                anyone.reference("*", "*"),
            );
        }
    }

    /** Store the owner of the accounts, a stranger and the operator, each shown to anyone. */
    static async #insertPeople(database: DatabaseConnection): Promise<void> {
        for (const person of PEOPLE) {
            // store the user
            await database.insert(user.table).values({
                ...RECORD_TIMES,
                ...person,
                ...SIGN_UP,
                emailVerified: true,
                homeId: Account.home(Account.personal(person.id)),
            });

            // make the user the owner of its scope and show its profile to anyone
            const subject = principal.user.reference(Scope.universe.id, person.id);
            const access = new AccessFixture(database);
            await access.replicateScope(subject);
            await access.bindOwner(subject, subject);
            for (const relationship of User.creation(subject).relationships) {
                await access.replicateRelationship(
                    person.id,
                    subject,
                    relationship.relation,
                    relationship.subject,
                );
            }
        }
    }

    /** Declare the platform with a cloud machine per residency. */
    static async #platformDefinition(
        cloudKeychains: Map<Identifier<"machine">, MemoryKeychain>,
    ): Promise<OperatorDefinition> {
        return {
            name: "Destack",
            residencyId: "eu",
            operator: OPERATOR_EMAIL,
            reserved: [],
            publishers: [],
            handle: PLATFORM_HANDLE,
            machines: [
                await cloudMachine(cloudKeychains, ids.machine, "eu-central", "eu"),
                await cloudMachine(cloudKeychains, ids.usMachine, "us-east", "us"),
            ],
        };
    }

    /** Serve the account and setting packages over a database with some keys. */
    static async #serve(
        storage: TestDatabase,
        keys: AccountKeys,
        options: ServeOptions,
        platform?: OperatorDefinition,
    ): Promise<Served> {
        // compose the universe's store of the account and setting packages, signing in once it exists
        const { database } = storage;
        const clock = options.signIn?.clock ?? Clock.system;
        const releases = new Map<string, Promise<BuildReader>>();
        const signIn = Promise.withResolvers<BetterAuthAuthenticator>();
        const events = await testEvents(database, [], clock);
        const opening = options.openRelease ?? AccountFixture.#opening(releases);
        const store = new ObjectStore({
            servers: [
                objectServer,
                accountServer({
                    resolver: { txt: async () => [] },
                    stacks: options.stacks,
                    signIn: signIn.promise,
                }),
                settingServer({
                    declared: Setting.declared((value) => opening(value.packageId, value.release)),
                    after: [],
                }),
            ],
            chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
            principal: Scope.universe,
            partition: Scope.universe.id,
            policies: options.policies ?? [],
            database,
            claims: { object: claim },
            clock,
            callKey: testCallKey,
            events,
            origin: { package: Package.of(accountPackage) },
            // follow the residencies' homes through a publisher that sends nothing, as no finance runs here
            publisher: () => TestEntitlement.publisher,
        });
        const { packages } = store;

        // own the universe as the universe, then store the platform before its identity
        await UniverseRole.trust(store, clock.now());
        if (platform !== undefined) {
            await new Operator(platform, options.stacks.residency).apply(store, clock.now());
        }

        // sign in with the universe's identity, mailing links and codes into an inbox
        const inbox = new Inbox();
        const authenticate = async (keyring: Keyring) =>
            BetterAuthAuthenticator.create({
                keystore: await IdentityKeystore.universe(
                    keyring,
                    database,
                    IdentityRegistry.of(store),
                    clock,
                ),
                database,
                callKey: testCallKey,
                issuer: ISSUER,
                trustedOrigins: [ORIGIN],
                ipAddress: { disableIpTracking: true },
                providers: {},
                objects: store,
                events,
                lifetimes: new Map(),
                mail: inbox,
                clock,
                ...options.signIn,
            });
        const authenticator = await authenticate(await LocalKeyring.open(keys.universe, "account"));
        signIn.resolve(authenticator);

        // serve each package to the callers the fixture verifies and to access tokens
        const callers = new Map<string, Verification>();
        const servers = new Map(
            packages.map((implementation) => [
                implementation.package.id,
                AccountFixture.serve(implementation, database, callers, clock),
            ]),
        );
        for (const served of servers.values()) {
            await AccountFixture.#serving(served);
        }

        return { objects: store, authenticator, authenticate, servers, callers, releases, inbox };
    }

    /** Open the releases a test publishes, by package and version, refusing any other. */
    static #opening(
        releases: ReadonlyMap<string, Promise<BuildReader>>,
    ): (packageId: PackageId, version: string) => Promise<BuildReader> {
        return async (packageId, version) => {
            const reader = releases.get(`${packageId}@${version}`);
            if (reader === undefined) {
                throw new CallError("NOT_FOUND", {
                    message: `no release ${version} of ${packageId}`,
                });
            }

            return reader;
        };
    }

    /** Wait until a served package serves, its replicas synced. */
    static async #serving(server: Server): Promise<void> {
        for await (const { status } of server.health.watch()) {
            if (status === "serving") {
                return;
            }
        }
    }

    /** Read the platform's account, which the seed stores. */
    static async #platform(database: DatabaseConnection): Promise<Identifier<"account">> {
        const [row] = await database
            .select({ id: account.table.id })
            .from(account.table)
            .where(eq(account.table.handle, PLATFORM_HANDLE));

        return present(row, "the platform's account").id;
    }

    /** Read the personal account a person's sign-up created, with its handle. */
    async #accountOf(
        userId: Identifier<"user">,
    ): Promise<{ readonly account: Identifier<"account">; readonly handle: string }> {
        const [personal] = await this.database
            .select({ account: account.table.id, handle: account.table.handle })
            .from(account.table)
            .where(and(eq(account.table.scope, userId), eq(account.table.kind, "personal")));

        return present(personal, `the personal account of ${userId}`);
    }

    /** Place a space in an account of a person on a machine the person enrolls in it. */
    async #place(
        person: Person,
        target: { readonly id: Identifier<"account">; readonly handle: string },
        spaceId: Identifier<"space">,
    ) {
        // enroll a machine, then create the space in the account, placed on it as creation places it
        const serving = this.caller(await this.enroll(target.id, { client: person.client }));
        await person.client.space.create({
            accountId: target.id,
            id: spaceId,
            requestId: RequestId.create(this.objects.clock.now()),
            name: `space-${spaceId.slice(-12)}`,
            machine: schema.identifier("machine").parse(serving.id),
        });

        return { accountId: target.id, handle: target.handle, spaceId, machine: serving };
    }

    /** List the replicas an installation requests now from the account package, its uplink. */
    static async #requests(objects: ObjectStore): Promise<Subscription[]> {
        const subscriber = present(objects.subscriber, "the installation's subscriber");

        return (await subscriber.subscriptions(Snapshot.live(objects.database)))
            .filter((entry) => entry.publisher === subscriber.uplink)
            .map((entry) => entry.subscription);
    }

    /** Create the owner's eu accounts as the owner. */
    async #createAccounts(): Promise<void> {
        const owner = principal.user.reference(Scope.universe.id, ids.owner);
        await this.objects.mutate(AccessContext.of(Caller.of(owner), this.objects.clock.now()), {
            id: RequestId.create(this.objects.clock.now()),
            calls: ACCOUNTS.map((declared) =>
                account.call("create", {
                    ...account.target({ scope: ids.owner, id: declared.id }),
                    handle: declared.handle,
                    name: declared.handle,
                    kind: declared.kind,
                    residencyId: "eu",
                }),
            ),
        });
    }

    /** Seat the operator as the platform organisation's owner through the invitation it stores. */
    async #seatOperator(): Promise<void> {
        // sign the operator in and find the platform organisation
        const operator = await this.session(ids.operator);
        const organisationId = present(
            await UniverseRole.operating(Snapshot.live(this.database)),
            "the platform organisation",
        );

        // accept the invitation the platform stores for the operator
        const [invited] = (await operator.user.invited({ id: ids.operator })).items;
        await operator.organisation.accept({
            id: organisationId,
            invitationId: present(invited, "the invitation to operate").id,
            requestId: RequestId.create(this.objects.clock.now()),
        });
    }

    /** Start the identities of the residencies' homes as the cloud machines serving them. */
    async #serveHomes(): Promise<void> {
        for (const entry of await Residency.list(this.database)) {
            const home = await this.home(entry.id);
            const serving = await this.serving(home);
            const registry = IdentityRegistry.client(this.machine(serving), this.objects.clock);
            const keystore = new IdentityKeystore(
                this.keystore.keyring,
                registry,
                this.objects.clock,
            );
            await keystore.generate(this.database, home);
        }
    }

    /** Stop serving the packages, then close the database. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
        await this.storage.close();
    }
}

/** How a test enrolls a machine: as whom and under which name. */
export interface MachineEnrollment {
    /** The client enrolling the machine. */
    readonly client?: ClientOf<typeof accountPackage>;
    /** The proposed name. */
    readonly name?: string;
    /** The residency a cloud machine serves. */
    readonly residencyId?: ResidencyCode;
}

/** A person signed in: their user and the typed client acting as them. */
export interface Person {
    /** The typed client acting as the person through a session's access tokens. */
    readonly client: ClientOf<typeof accountPackage>;
    /** The person's session token, which the token endpoint exchanges. */
    readonly session: string;
    /** The user's identifier. */
    readonly id: Identifier<"user">;
    /** The user as a principal. */
    readonly subject: Subject;
    /** The person's personal account, which their sign-up created. */
    readonly account: Identifier<"account">;
    /** The handle of the person's personal account. */
    readonly handle: string;
}

/** A caller the fixture verifies at each request: its credential and subject. */
export interface Verification {
    /** The credential's kind, such as a space key. */
    readonly kind: string;
    /** The credential's identifier. */
    readonly id: string;
    /** The subject it verifies. */
    readonly subject: Subject;
}

/** A machine, space or installation a test acts as: its identity, principal and typed client. */
export interface Actor {
    /** The caller's identity. */
    readonly id: string;
    /** The caller as a principal. */
    readonly subject: Subject;
    /** The typed client acting as the caller. */
    readonly client: ClientOf<typeof accountPackage>;
}

/** The email sign-in mailed, stored as the platform's account would send it. */
export class Inbox implements MailTransport {
    /** The mailed emails, oldest first. */
    readonly #mail: { readonly email: string; readonly text: string }[] = [];

    /** Store an email sign-in mails to an address. */
    async send(mail: Mail): Promise<MailOutcome> {
        this.#mail.push({ email: mail.to, text: mail.text });

        return { kind: "sent" };
    }

    /** Read the sign-in links mailed so far, oldest first. */
    async links(): Promise<{ readonly email: string; readonly url: string }[]> {
        return this.#mail.flatMap(({ email, text }) => {
            const [url] = /https?:\/\/\S+/u.exec(text) ?? [];

            return url === undefined ? [] : [{ email, url }];
        });
    }

    /** Read the one-use codes mailed so far with what each is for, oldest first. */
    async codes(): Promise<
        { readonly email: string; readonly otp: string; readonly purpose: string }[]
    > {
        return this.#mail.flatMap(({ email, text }) => {
            const [, purpose, otp] = /Your Destack (.+) code is (\S+)\./u.exec(text) ?? [];

            return purpose === undefined || otp === undefined ? [] : [{ email, otp, purpose }];
        });
    }
}

/** Make a free handle for a person or an account a test creates. */
function freshHandle(): string {
    return `account-${Identifier.create("account").slice(-12)}`;
}

/** Verify a fixture caller at a time, for a token's lifetime toward an audience. */
function verified(verification: Verification, audience: PackageId, now: number): Authentication {
    const { kind, id, subject } = verification;

    return {
        caller: Caller.of(subject),
        credential: {
            kind,
            id,
            audience,
            verifiedAt: now,
            expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
        },
    };
}

/** Declare a cloud machine of the universe serving a residency, storing its keychain. */
async function cloudMachine(
    keychains: Map<Identifier<"machine">, MemoryKeychain>,
    id: Identifier<"machine">,
    name: string,
    residencyId: ResidencyCode,
): Promise<OperatorDefinition["machines"][number]> {
    // generate its key, storing its keychain for tests to act as it and run it
    const keychain = new MemoryKeychain();
    const { publicKey } = await new MachineIdentity(id, keychain, Clock.system).generate();
    keychains.set(id, keychain);

    return {
        id,
        name,
        residencyId,
        providerCode: PROVIDER,
        location: LOCATION,
        key: publicKey,
        origin: `https://${name}.machine.test/`,
    };
}

/** The keys of the universe and the declared cloud machines, which each seed copy shares. */
class AccountKeys {
    /** The keychains' secrets, as an account template's value carries them. */
    static readonly schema = schema.object({
        /** The universe keychain's secrets, by name. */
        universe: schema.record(schema.string(), schema.string()),
        /** Each cloud machine's keychain secrets, by machine and name. */
        cloud: schema.record(schema.string(), schema.record(schema.string(), schema.string())),
    });

    /** The keychain of the universe's keyring. */
    readonly universe: MemoryKeychain;
    /** The keychains of the declared cloud machines, by machine. */
    readonly cloud: Map<Identifier<"machine">, MemoryKeychain>;

    /** Store the keychains. */
    constructor(universe: MemoryKeychain, cloud: Map<Identifier<"machine">, MemoryKeychain>) {
        this.universe = universe;
        this.cloud = cloud;
    }

    /** Start empty keychains. */
    static create(): AccountKeys {
        return new AccountKeys(new MemoryKeychain(), new Map());
    }

    /** Open keychains holding the secrets of a template's value. */
    static of(value: AccountKeysValue): AccountKeys {
        const cloud = Object.entries(value.cloud).map(
            ([machineId, secrets]) =>
                [
                    schema.identifier("machine").parse(machineId),
                    AccountKeys.#keychain(secrets),
                ] as const,
        );

        return new AccountKeys(AccountKeys.#keychain(value.universe), new Map(cloud));
    }

    /** Read the keychains' secrets as a template's value carries them. */
    value(): AccountKeysValue {
        return {
            universe: Object.fromEntries(this.universe.secrets),
            cloud: Object.fromEntries(
                [...this.cloud].map(([machineId, keychain]) => [
                    machineId,
                    Object.fromEntries(keychain.secrets),
                ]),
            ),
        };
    }

    /** Open a keychain holding some secrets. */
    static #keychain(secrets: Readonly<Record<string, string>>): MemoryKeychain {
        const keychain = new MemoryKeychain();
        for (const [name, secret] of Object.entries(secrets)) {
            keychain.secrets.set(name, secret);
        }

        return keychain;
    }
}

/** The keychains' secrets, as an account template's value carries them. */
type AccountKeysValue = schema.Infer<typeof AccountKeys.schema>;

/** The account fixture's rows with the default stacks and the fixture's homes, which tests copy. */
export const ACCOUNT_TEMPLATE = AccountFixture.template({
    name: "account",
    world: { value: schema.null(), write: async () => null },
});

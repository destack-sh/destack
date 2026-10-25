import type { Environment, HostCapability } from "@destack/environment";
import { eq, type DatabaseConnection, type Dialect, Snapshot } from "@destack/db";
import { type ImageIndex, TestDatabase, TestTemplate } from "@destack/db/test";
import { Scope, Subject, RequestId } from "@destack/sync";
import {
    AccessContext,
    Authorization,
    Caller,
    InstallationId,
    type Policy,
    principal,
    type Relationship,
} from "@destack/access";
import {
    Jwks,
    Jwt,
    type JwtClaims,
    type Keychain,
    type Keyring,
    LocalKeyring,
    MemoryKeychain,
} from "@destack/identity";
import { claim } from "../object/claim.ts";
import { placement } from "../object/placement.ts";
import objectPackage from "@destack/object/package";
import objectServer, { ObjectStore } from "@destack/object/server";
import {
    testEvents,
    testCallKey,
    type TestCaller,
    TestObjectStore,
    type TestSigner,
} from "@destack/object/test";
import { Package, PackageId } from "@destack/package";
import { ENTITLED_PACKAGE } from "./entitlement.ts";
import type { BuildReader } from "@destack/package/manifest";
import { Digest, found, Identifier, type JsonValue, present, schema } from "@destack/schema";
import {
    CallError,
    type FeatureReference,
    type Fetch,
    Installation,
    PackageMount,
} from "@destack/object";
import { Setting } from "@destack/setting";
import { settingServer } from "@destack/setting/server";
import settingPackage from "@destack/setting/package";
import {
    AccountAuthentication,
    BetterAuthAuthenticator,
    type BetterAuthAuthenticatorOptions,
    type SignUp,
} from "../better-auth/index.ts";
import { MachineIdentity } from "../client/index.ts";
import { Issuer } from "../native/index.ts";
import { IdentityKeystore, IdentityRegistry, type TokenSource } from "../identity/index.ts";
import {
    account,
    Account,
    entitlement,
    Entitlement,
    type EntitlementResource,
    machine,
    type Membership,
    organization,
    ProviderCode,
    RESIDENCY_RELAY,
    Residency,
    type ResidencyCode,
    type ResidencyDefinition,
    SpaceKind,
    StackSource,
} from "../object/index.ts";
import accountPackage, { OAUTH_PATH } from "../package.ts";
import { accountServer, Platform, PlatformDefinition, UniverseRole } from "../server/index.ts";
import { accountDatabase } from "../stack/index.ts";
import { TokenLifetime } from "../oauth/index.ts";
import { TestBrowser } from "./browser.ts";
import { TestDns } from "./dns.ts";
import type { DnsResolver } from "../server/dns-resolver.ts";
import { freshHandle, TestPerson, type TestSignIn } from "./person.ts";
import { authenticatorCode, Enrollment } from "./authenticator.ts";
import { sudo } from "../authentication/sudo.ts";
import type { Mail, MailOutcome, MailTransport } from "../better-auth/index.ts";
import { Simulation, type SimulatedClock } from "@destack/environment/simulation";
import { Pkce } from "@destack/access/token";

/** The universe's origin. */
export const ORIGIN = "http://localhost:3210";

/** The loopback redirect a stepped-up native client returns to, which the step-up stops at. */
const STEP_UP_REDIRECT = "http://127.0.0.1:53682/callback";

/** The redirects a browser follows at most, the Fetch standard's limit. */
const FETCH_REDIRECTS = 20;

/** The seed of the simulated world a fixture runs in. */
const WORLD_SEED = 1;

/** The moment a fixture's world starts at, paused: 2026-10-01T00:00:00Z. */
const WORLD_START = Date.UTC(2026, 9, 1);

/** The node the universe's calls run as in the fixture's world. */
const WORLD_NODE = "universe";

/** The packages the fixture's universe serves, each with its base installation. */
const UNIVERSE_PACKAGES = [objectPackage, accountPackage, settingPackage];

/** The button a person presses on each page a step-up passes. */
const STEP_UP_ACTIONS = [["/consent", "approve"]] as const;

/** The URL the fixture's users, machines and installations call the account package at. */
const ACCOUNTS_URL = PackageMount.url(ORIGIN, accountPackage.id);

/** The universe's issuer: the account package's oauth adapter. */
export const ISSUER = `${ACCOUNTS_URL}${OAUTH_PATH}`;

/** The email of the person operating the fixture's universe. */
export const OPERATOR_EMAIL = "operator@example.com";

/** The email of the person owning the fixture's accounts, the handle acme's. */
export const OWNER_EMAIL = "acme@example.test";

/** The email of a person outside the fixture's accounts. */
export const STRANGER_EMAIL = "stranger@example.test";

/** The time zone and locale the fixture's people sign up with. */
export const SIGN_UP: Pick<SignUp, "timeZone" | "locale"> = {
    timeZone: "Europe/Vienna",
    locale: "en",
};

/** The handle of the platform's account, whose home serves its residency. */
export const PLATFORM_HANDLE = "destack";

/** The owner of the fixture's accounts. */
const OWNER = schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000a");

/** The identities the account package stores from its start. */
export const ids = {
    owner: OWNER,
    stranger: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000b"),
    operator: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000c"),
    founder: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000d"),
    administrator: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000e"),
    member: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000f"),
    assistant: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000010"),
    partner: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000011"),
    person: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000012"),
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
    account: {
        package: Package.parse({
            id: "package-01996ab0-0000-7000-8000-0000000000b4",
            name: "@example/account",
            version: "2026.9.0",
        }),
        manifest: Digest.parse("e".repeat(64)),
        export: "ACCOUNT",
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

/** The residencies of the universe, as the platform's bootstrap declares them. */
const RESIDENCIES: readonly ResidencyDefinition[] = [
    { code: "eu", name: "European Union" },
    { code: "us", name: "United States" },
];

/** The people who sign up at the account package's start, and whether they step up. */
export const PEOPLE = [
    { id: ids.owner, name: "Owner", email: OWNER_EMAIL, handle: "acme", isEnrolled: true },
    {
        id: ids.stranger,
        name: "Stranger",
        email: STRANGER_EMAIL,
        handle: "stranger",
        isEnrolled: false,
    },
    {
        id: ids.operator,
        name: "Platform",
        email: OPERATOR_EMAIL,
        handle: "operator",
        isEnrolled: true,
    },
    {
        id: ids.founder,
        name: "Founder",
        email: "founder@example.com",
        handle: "founder",
        isEnrolled: true,
    },
    {
        id: ids.administrator,
        name: "Administrator",
        email: "administrator@example.com",
        handle: "administrator",
        isEnrolled: true,
    },
    {
        id: ids.member,
        name: "Member",
        email: "member@example.com",
        handle: "member",
        isEnrolled: true,
    },
    {
        id: ids.assistant,
        name: "Assistant",
        email: "assistant@example.com",
        handle: "assistant",
        isEnrolled: true,
    },
    {
        id: ids.partner,
        name: "Partner",
        email: "partner@example.com",
        handle: "partner",
        isEnrolled: false,
    },
    {
        id: ids.person,
        name: "Person",
        email: "person@example.com",
        handle: "person",
        isEnrolled: false,
    },
] as const;

/** A person's client, which their sign-ins refresh, and the authenticator they step up with. */
const TestSession = schema.object({
    /** The person's user. */
    userId: schema.identifier("user"),
    /** The handle of the person's personal account. */
    handle: schema.string(),
    /** The client the person signed in on. */
    clientId: schema.identifier("client"),
    /** The client's current refresh token. */
    refreshToken: schema.string(),
    /** The URI the person's authenticator enrolled from. */
    authenticator: schema.string(),
});
/** A person's client, which their sign-ins refresh, and the authenticator they step up with. */
type TestSession = schema.Infer<typeof TestSession>;

/** The owner's further accounts the account package stores from its start. */
const ACCOUNTS = [{ id: ids.other, handle: "rival", kind: "shared" }] as const;

/** How a test serves the account package. */
export interface TestAccountStoreOptions {
    /** Open the release declaring a setting value. */
    readonly openRelease?: (packageId: PackageId, version: string) => Promise<BuildReader>;
    /** The policies of other packages' types. */
    readonly policies?: readonly Policy[];
    /** The sign-in options the test changes. */
    readonly signIn?: Partial<BetterAuthAuthenticatorOptions>;
    /** The template images to copy from, the run's when absent. */
    readonly images?: ImageIndex;
    /** The DNS domain claims verify through, none answering by default. */
    readonly resolver?: DnsResolver;
    /** The database the images are of, the test project's when absent. */
    readonly dialect?: Dialect;
}

/** How a test serves the account package over its template's stacks and homes, in its world. */
type ServeOptions = TestAccountStoreOptions & {
    /** The world the store runs in. */
    readonly simulation: Simulation;
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
    readonly serve?: TestAccountStoreOptions;
    /** Write a world on the fixture's rows, returning what its copies share. */
    readonly world: {
        /** What the world's copies share, as its JSON form crosses to the tests. */
        readonly value: schema.Schema<World>;
        /** Write the world over the served fixture, storing files in a run's directory. */
        readonly write: (accounts: TestAccountStore, directory: string) => Promise<World>;
    };
}

/** What an account template's copies share: its write's keys, stacks and homes, and its world. */
export type AccountTemplateValue<World extends JsonValue> = {
    /** The keychains' secrets the write ended with. */
    readonly keys: AccountKeysValue;
    /** The sessions of the people with an authenticator, by email. */
    readonly sessions: Readonly<Record<string, TestSession>>;
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
    readonly store: ObjectStore;
    /** The universe's sign-in. */
    readonly authenticator: BetterAuthAuthenticator;
    /** Open another sign-in over the same store and mail with a keyring. */
    readonly authenticate: (keyring: Keyring) => Promise<BetterAuthAuthenticator>;
    /** The universe's packages served over HTTP to bearers, signed by each caller's key. */
    readonly served: TestObjectStore;
    /** The machines tests act as, by machine. */
    readonly machines: Map<string, MachineIdentity>;
    /** The published releases declaring settings, by package and version. */
    readonly releases: Map<string, Promise<BuildReader>>;
    /** The email sign-in mailed. */
    readonly mail: TestMail;
    /** The issuer as native clients call it. */
    readonly issuer: Issuer;
    /** Route a request of the universe's origin to the package mounted at its path. */
    readonly fetch: (request: Request) => Promise<Response>;
}

/** The account package in one database, served as the universe serves it, with its people. */
export class TestAccountStore implements AsyncDisposable {
    /** The account package's test database. */
    readonly storage: TestDatabase;
    /** The account database. */
    readonly database: DatabaseConnection;
    /** The sign-in over the database. */
    readonly authenticator: BetterAuthAuthenticator;
    /** The account and setting packages served over HTTP, running their controllers. */
    readonly served: TestObjectStore;
    /** The universe's object store. */
    readonly store: ObjectStore;
    /** The world the store runs in, its clock flowing from the system's time until paused. */
    readonly simulation: Simulation;
    /** The keys of the universe and of the residencies' home spaces, signing their tokens. */
    readonly keystore: IdentityKeystore;
    /** The issuer as native clients call it: sign-in, refresh and token exchange. */
    readonly issuer: Issuer;
    /** The published releases declaring settings, by package and version. */
    readonly releases: Map<string, Promise<BuildReader>>;
    /** The platform's account, whose home serves its residency. */
    readonly platform: Identifier<"account">;
    /** Who serves the residencies' homes. */
    readonly homes: HomeServer;
    /** The email sign-in mailed. */
    readonly #mail: TestMail;
    /** The authenticator enrollments of the people who step up, by user. */
    readonly #authenticators: Map<Identifier<"user">, string>;
    /** The sessions of the people with an authenticator, by email, refreshed at each sign-in. */
    readonly #sessions: Map<string, TestSession>;
    /** The machines tests act as: the declared and the enrolled ones, by machine. */
    readonly #machines: Map<string, MachineIdentity>;
    /** The keychains storing the keys of the declared cloud machines, by machine. */
    readonly #cloudKeychains: ReadonlyMap<Identifier<"machine">, Keychain>;
    /** The keychain of the universe's keyring. */
    readonly #keychain: MemoryKeychain;
    /** Open another sign-in over the same store and mail with a keyring. */
    readonly #authenticate: (keyring: Keyring) => Promise<BetterAuthAuthenticator>;
    /** Route a request of the universe's origin to the package mounted at its path. */
    readonly #fetch: (request: Request) => Promise<Response>;

    /** Store a served account package. */
    private constructor(
        storage: TestDatabase,
        served: Served,
        state: {
            /** The platform's account. */
            readonly platform: Identifier<"account">;
            /** Who serves the residencies' homes. */
            readonly homes: HomeServer;
            /** The world the store runs in. */
            readonly simulation: Simulation;
            /** The keychains storing the keys of the declared cloud machines, by machine. */
            readonly cloudKeychains: ReadonlyMap<Identifier<"machine">, Keychain>;
            /** The keychain of the universe's keyring. */
            readonly keychain: MemoryKeychain;
            /** The sessions of the people with an authenticator, by email. */
            readonly sessions: Readonly<Record<string, TestSession>>;
        },
    ) {
        // store the served store, sign-in and servers over the database
        this.storage = storage;
        this.database = storage.database;
        this.store = served.store;
        this.authenticator = served.authenticator;
        this.served = served.served;
        this.keystore = served.authenticator.keystore;

        // store what tests read and change
        this.issuer = served.issuer;
        this.releases = served.releases;
        this.platform = state.platform;
        this.homes = state.homes;
        this.simulation = state.simulation;
        this.#mail = served.mail;
        this.#machines = served.machines;
        this.#cloudKeychains = state.cloudKeychains;
        this.#keychain = state.keychain;
        this.#sessions = new Map(Object.entries(state.sessions));
        this.#authenticators = new Map(
            Object.values(state.sessions).map((session) => [session.userId, session.authenticator]),
        );
        this.#authenticate = served.authenticate;
        this.#fetch = served.fetch;
    }

    /** The clock the store runs at, its world's. */
    get clock(): SimulatedClock {
        return this.simulation.clock;
    }

    /** The world the store's calls run in, as the universe's node. */
    get environment(): Environment<HostCapability> {
        return this.simulation.environment(WORLD_NODE);
    }

    /** Define a template of the fixture's rows, and of a world a further write puts on them. */
    static template<World extends JsonValue>(
        options: AccountTemplateOptions<World>,
    ): TestTemplate<AccountTemplateValue<World>, AccountTemplateDatabases> {
        // write the fixture's rows with the template's stacks and homes
        const stacks = options.stacks ?? STACKS;
        const homes = options.homes ?? "fixture";
        const world = options.world;

        return new TestTemplate({
            name: options.name,
            databases: { universe: accountDatabase },
            ...(options.dialects === undefined ? {} : { dialects: options.dialects }),
            value: schema.object({
                keys: AccountKeys.schema,
                sessions: schema.record(schema.string(), TestSession),
                stacks: schema.record(SpaceKind, StackSource),
                homes: schema.enum(["fixture", "machine"]),
                world: world.value,
            }),
            world: { seed: WORLD_SEED, start: WORLD_START },
            write: async ({ universe: storage }, directory, simulation) => {
                // write and serve the fixture's rows, then the world over them
                const served = {
                    ...options.serve,
                    stacks,
                    homes,
                    simulation,
                };
                const { keys, sessions } = await TestAccountStore.#write(storage, served);
                const accounts = await TestAccountStore.#over(
                    storage,
                    await TestAccountStore.#serve(storage, keys, served),
                    keys,
                    { ...served, sessions },
                );
                const value = await world.write(accounts, directory);
                await accounts.close();

                return { keys: keys.value(), sessions, stacks, homes, world: value };
            },
        });
    }

    /** Write and serve the fixture's rows in a fresh database, as a simulation builds one. */
    static async create(
        options: TestAccountStoreOptions &
            Partial<Pick<AccountTemplateOptions<null>, "stacks" | "homes">> = {},
    ): Promise<TestAccountStore> {
        // write the fixture's rows with the given stacks and homes, the defaults when absent
        const simulation = TestAccountStore.#world();
        const storage = await TestDatabase.create(accountDatabase, {
            environment: simulation.environment(WORLD_NODE),
            isMigrated: true,
        });
        const { stacks = STACKS, homes = "fixture", ...rest } = options;
        const served = { ...rest, stacks, homes, simulation };
        const { keys, sessions } = await TestAccountStore.#write(storage, served);

        // serve them
        return await TestAccountStore.#over(
            storage,
            await TestAccountStore.#serve(storage, keys, served),
            keys,
            { ...served, sessions },
        );
    }

    /** Serve the account package with the owner's accounts and the platform's organization. */
    static async open(options: TestAccountStoreOptions = {}): Promise<TestAccountStore> {
        return (await TestAccountStore.copy(ACCOUNT_TEMPLATE, options)).accounts;
    }

    /** Serve a copy of a template's rows, with the world its write put on them. */
    static async copy<World extends JsonValue>(
        template: TestTemplate<AccountTemplateValue<World>, AccountTemplateDatabases>,
        options: TestAccountStoreOptions = {},
    ): Promise<{ readonly accounts: TestAccountStore; readonly world: World }> {
        // copy the template into a fork of its world
        const { simulation, databases, value } = await TestDatabase.copy(template, {
            label: "copy",
            ...(options.images === undefined ? {} : { images: options.images }),
            ...(options.dialect === undefined ? {} : { dialect: options.dialect }),
        });

        // serve it with the keys, stacks and homes the template's write ended with
        const keys = AccountKeys.of(value.keys);
        const { stacks, homes, sessions } = value;
        const serving = { ...options, stacks, homes, simulation };
        const storage = databases.universe;
        const served = await TestAccountStore.#serve(storage, keys, serving);

        return {
            accounts: await TestAccountStore.#over(storage, served, keys, { ...serving, sessions }),
            world: value.world,
        };
    }

    /** Read a residency's space, the home of the platform's account in it. */
    async residencySpace(residencyId: ResidencyCode): Promise<Identifier<"space">> {
        return Residency.space(Snapshot.live(this.database), residencyId);
    }

    /** Route a request of the universe's origin to the package mounted at its path. */
    fetch(request: Request): Promise<Response> {
        return this.#fetch(request);
    }

    /** Sign a person in on their stored client, else up or in with a mailed code on a new one. */
    async signIn(email: string): Promise<TestPerson> {
        // read the stored client of a person with an authenticator
        const session = this.#sessions.get(email);

        // sign a person without an authenticator up or in with a mailed code, as the CLI does
        if (session === undefined) {
            return TestPerson.signIn(this.#signIn(), email);
        }

        // refresh the client of a person with one, whom a mailed code no longer signs in
        const tokens = await this.issuer.refresh(session.clientId, session.refreshToken);
        this.#sessions.set(email, { ...session, refreshToken: tokens.refreshToken });

        return new TestPerson(this.#signIn(), {
            id: session.userId,
            email,
            handle: session.handle,
            clientId: session.clientId,
            tokens,
        });
    }

    /** Sign a person up with an eu personal account through a mailed link in a new browser. */
    async signInWithLink(
        email: string,
        handle: string = freshHandle(this.environment),
    ): Promise<{ readonly person: TestPerson; readonly browser: TestBrowser }> {
        // follow the mailed link
        const browser = this.browser();
        await browser.fetch("/sign-in/magic-link", {
            email,
            callbackURL: "/",
            metadata: { ...SIGN_UP, name: email, handle, residencyId: "eu" },
        });
        await browser.fetch(await this.link());

        // sign the same person in on a native client too, for calls beside the browser's pages
        return { person: await this.signIn(email), browser };
    }

    /** Start a space's identity as the machine serving it, its keys in the fixture's keystore. */
    async startIdentity(spaceId: Identifier<"space">, serving: TestCaller): Promise<void> {
        const registry = IdentityRegistry.client(serving.client(accountPackage), this.environment);
        const keystore = new IdentityKeystore(this.keystore.keyring, registry, this.environment);
        await keystore.generate(this.database, spaceId);
    }

    /** Call out as a space or installation by its space's key, as the machine serving it does. */
    source(subject: Subject): TokenSource {
        return this.keystore.source(this.database, subject, {
            environment: this.environment,
            issuer: ISSUER,
            send: (request) => this.fetch(request),
        });
    }

    /** Step a person up with their authenticator in a browser on a new client, as RFC 9470 does. */
    async elevate(person: TestPerson, browser: TestBrowser = this.browser()): Promise<TestPerson> {
        // enroll an authenticator the first time, in a browser the person's link signs in
        const enrolled =
            this.#authenticators.get(person.id) ?? (await this.#enroll(person, this.browser()));

        // ask the issuer to sign the person in again with their second factor, as a step-up does
        const clientId = Identifier.create("client", this.environment);
        const authorization = await this.issuer.authorize(
            clientId,
            STEP_UP_REDIRECT,
            "step-up",
            this.store.clock.now(),
            sudo,
        );
        const asked = new URL(TestBrowser.location(await browser.fetch(authorization.url)));

        // sign in by the mailed link, and answer the second-factor challenge with the authenticator's code
        await browser.fetch(
            asked.href,
            new URLSearchParams({ action: "link", email: person.email }),
        );
        const linked = await browser.fetch(await this.link());
        const challenge = TestBrowser.location(linked);
        const verified = await browser.fetch(
            challenge,
            new URLSearchParams({
                action: "totp",
                code: await authenticatorCode(enrolled, this.environment),
            }),
        );

        // follow the issuer back to the new client, continuing the sign-in and approving its consent
        let next = TestBrowser.location(verified);
        const passed: string[] = [];
        while (!next.startsWith(STEP_UP_REDIRECT)) {
            const url = new URL(next);
            passed.push(`${url.pathname}${url.search}`);
            if (passed.length > FETCH_REDIRECTS) {
                throw new TypeError(`the step-up redirected too often: ${passed.join(" -> ")}`);
            }
            const action = STEP_UP_ACTIONS.find(([path]) => url.pathname.endsWith(path))?.[1];
            const answered = await browser.fetch(
                url.href,
                action === undefined ? undefined : new URLSearchParams({ action }),
            );
            next = TestBrowser.location(answered);
        }

        // redeem the code for the new client's tokens
        const code = present(new URL(next).searchParams.get("code"), "the authorization code");
        const { tokens } = await this.issuer.redeem(
            clientId,
            code,
            authorization.verifier,
            STEP_UP_REDIRECT,
        );

        return new TestPerson(this.#signIn(), {
            id: person.id,
            email: person.email,
            handle: person.handle,
            clientId,
            tokens,
        });
    }

    /** Enroll a person's authenticator in a browser their link signs in, returning its URI. */
    async #enroll(person: TestPerson, browser: TestBrowser): Promise<string> {
        // sign the browser in by the person's link
        await browser.fetch("/sign-in/magic-link", { email: person.email, callbackURL: "/" });
        await browser.fetch(await this.link());

        // enable an authenticator and verify the code it showed a step ago, leaving the current code to sign in with
        const enabled = Enrollment.parse(
            await (await browser.fetch("/two-factor/enable", {})).json(),
        );
        await browser.fetch("/two-factor/verify-totp", {
            code: await authenticatorCode(enabled.totpURI, this.environment, -1),
        });
        this.#authenticators.set(person.id, enabled.totpURI);

        return enabled.totpURI;
    }

    /** Open another sign-in over the database, the universe's root key rotated, for browsers. */
    async rotate(): Promise<Fetch> {
        // open the sign-in on the rotated root key
        const keyring = await LocalKeyring.rotate(this.#keychain, "account", this.environment);
        const authenticator = await this.#authenticate(keyring);

        // send each request with its path below the issuer's, as the oauth adapter mounts it
        const prefix = new URL(ISSUER).pathname;

        return (request) => {
            const url = new URL(request.url);
            const below = `${ORIGIN}${url.pathname.slice(prefix.length)}${url.search}`;

            return authenticator.handler(new Request(below, request));
        };
    }

    /** Read the sign-in links mailed so far, oldest first. */
    links(): ReturnType<TestMail["links"]> {
        return this.#mail.links();
    }

    /** Read the one-use codes mailed so far with what each is for, oldest first. */
    codes(): ReturnType<TestMail["codes"]> {
        return this.#mail.codes();
    }

    /** Read the last sign-in link the issuer mailed. */
    async link(): Promise<string> {
        const links = await this.#mail.links();

        return present(links.at(-1), "a mailed link").url;
    }

    /** Read the last one-use code the issuer mailed. */
    async code(): Promise<string> {
        const codes = await this.#mail.codes();

        return present(codes.at(-1), "a mailed code").otp;
    }

    /** Open a browser of the issuer's pages, sent through the universe or another sign-in. */
    browser(
        language: string = SIGN_UP.locale,
        send: Fetch = (request) => this.fetch(request),
    ): TestBrowser {
        return new TestBrowser(ISSUER, send, language);
    }

    /** Authorize a client with PKCE in a signed-in browser, approve it, and redeem its code. */
    async authorize(browser: TestBrowser, client: TestAuthorization): Promise<Response> {
        // authorize with a fresh verifier, which sends the signed-in browser to the consent page
        const verifier = Pkce.verifier(this.environment.random);
        const resource = client.resource === null ? {} : { resource: client.resource };
        const query = new URLSearchParams({
            response_type: "code",
            client_id: client.clientId,
            redirect_uri: client.redirectUri,
            scope: client.scope,
            state: client.state,
            code_challenge: await Pkce.challenge(verifier, this.environment.crypto),
            code_challenge_method: "S256",
            ...resource,
        });
        const consent = TestBrowser.location(await browser.fetch(`/oauth2/authorize?${query}`));

        // approve on the consent page, which returns the browser to the client with a code
        const approved = await browser.fetch(consent, new URLSearchParams({ action: "approve" }));
        const callback = new URL(TestBrowser.location(approved));
        const code = present(callback.searchParams.get("code"), "the authorization code");

        // redeem the code with its verifier as the client
        const form = new URLSearchParams({
            grant_type: "authorization_code",
            code,
            redirect_uri: client.redirectUri,
            client_id: client.clientId,
            code_verifier: verifier,
            ...resource,
        });

        return this.fetch(new Request(`${ISSUER}/oauth2/token`, { method: "POST", body: form }));
    }

    /** Verify an ID token with the keys the issuer publishes, returning its claims. */
    async verifyIdToken(token: string): Promise<JwtClaims> {
        // verify it by the issuer's keys at the store's time
        const { crypto } = this.environment;
        const keys = Jwks.local(await this.authenticator.keys(), crypto);
        const options = { algorithms: ["ES256"], issuer: ISSUER, now: this.clock.now() } as const;

        return (await Jwt.verify(token, keys, options, crypto)).claims;
    }

    /** Create a new shared account of a person in a residency. */
    createAccount(person: TestPerson, residencyId: ResidencyCode = "eu"): Promise<Account> {
        return person.client(accountPackage).account.create({
            kind: "shared",
            scope: person.id,
            requestId: RequestId.create(this.store.environment),
            handle: freshHandle(this.environment),
            name: "Account",
            residencyId,
        });
    }

    /** Place a new space in a new shared account of a person, on a machine enrolled in it. */
    async createSpace(person: TestPerson, residencyId: ResidencyCode = "eu") {
        const created = await this.createAccount(person, residencyId);

        return this.#place(person, created, Identifier.create("space", this.environment));
    }

    /** Read a person's home, placed at sign-up, with the cloud machine serving it as a caller. */
    async personalHome(person: TestPerson) {
        const spaceId = Account.home(person.account);

        return {
            accountId: person.account,
            spaceId,
            machine: this.as(await this.machine(spaceId)),
        };
    }

    /** Entitle an account to a resource or feature as finance does, replacing its terms. */
    async entitle(
        accountId: Identifier<"account">,
        granted: EntitlementResource | { readonly feature: FeatureReference },
        terms: Pick<Entitlement, "kind"> & Partial<Pick<Entitlement, "value" | "limit" | "state">>,
    ): Promise<void> {
        // key a resource's entitlement by a test feature of its name
        const feature =
            typeof granted === "string"
                ? { packageId: ENTITLED_PACKAGE, name: granted, resource: granted }
                : { ...granted.feature, resource: null };

        // act as the account's residency home, whose installations provide entitlements
        const snapshot = Snapshot.live(this.database);
        const home = await Account.locate(snapshot, accountId);
        const provider = principal.space.reference(Scope.universe.id, home);
        const now = this.store.clock.now();

        // replace the one the account has with one of the terms
        const existing = await Entitlement.read(snapshot, accountId, granted);
        const created = entitlement.call("create", {
            ...entitlement.in(accountId),
            packageId: feature.packageId,
            feature: feature.name,
            resource: feature.resource,
            value: null,
            limit: null,
            state: null,
            ...terms,
        });
        await this.store.mutate(AccessContext.of(Caller.of(provider), now), {
            id: RequestId.create(this.environment),
            calls: [
                ...(existing === undefined
                    ? []
                    : [entitlement.call("delete", entitlement.target(existing))]),
                created,
            ],
        });
    }

    /** Act as the cloud machine a space is placed on. */
    async machine(spaceId: Identifier<"space">): Promise<MachineIdentity> {
        const placed = present(
            await Snapshot.live(this.database)
                .read(placement.table)
                .findFirst({ where: { id: spaceId } }),
            `the placement of ${spaceId}`,
        );

        return this.cloud(placed.machineId);
    }

    /** Read the relay a residency's home declares, which the universe relates to the residency. */
    async relay(residencyId: ResidencyCode): Promise<Subject> {
        // derive the installation the home's stack declares under the relay's name
        const snapshot = Snapshot.live(this.database);
        const home = await Scope.object(snapshot, await this.residencySpace(residencyId));
        const id = await InstallationId.declared(home, RESIDENCY_RELAY, this.environment.crypto);

        return principal.installation.reference(home.id, id);
    }

    /** Act as a cloud machine the platform declares, such as a residency's shared machine. */
    cloud(machineId: Identifier<"machine">): MachineIdentity {
        return found(this.#machines, machineId);
    }

    /** Read the keychain storing a declared cloud machine's key. */
    cloudKeychain(machineId: Identifier<"machine">): Keychain {
        return found(this.#cloudKeychains, machineId);
    }

    /** Add a person to an organization through an accepted invitation, returning the membership. */
    async join(
        inviter: TestPerson,
        person: TestPerson,
        organizationId: string,
        relation: Membership,
    ): Promise<Relationship> {
        // invite the person to the relation on the organization, and accept as them
        const object = organization.reference(Scope.universe.id, organizationId);
        const invitation = await inviter.client(objectPackage).invitation.create({
            scope: Scope.universe.id,
            requestId: RequestId.create(this.store.environment),
            object,
            relationship: { relation, subject: person.subject },
        });

        return await person.client(objectPackage).invitation.accept({
            scope: Scope.universe.id,
            requestId: RequestId.create(this.store.environment),
            object,
            invitationId: invitation.id,
        });
    }

    /** Enroll a new machine of an account as a person, and act as it from then on. */
    async enroll(
        accountId: Identifier<"account">,
        person: TestPerson,
        options: MachineEnrollment = {},
    ): Promise<MachineIdentity> {
        // enroll a new identity, acting as it from then on
        const identity = new MachineIdentity(
            Identifier.create("machine", this.environment),
            new MemoryKeychain(),
            this.environment,
        );
        await identity.enroll(person.client(accountPackage), {
            scope: accountId,
            requestId: RequestId.create(this.store.environment),
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
        this.#machines.set(identity.machineId, identity);

        return identity;
    }

    /** Declare an account's dedicated cloud machine under its plan, answering its identity. */
    async declareCloudMachine(
        accountId: Identifier<"account">,
        residencyId: ResidencyCode,
    ): Promise<MachineIdentity> {
        // entitle the account to dedicated tenancy, which the platform requires to declare its machine
        await this.entitle(accountId, "tenancy", {
            kind: "static",
            value: ["shared", "dedicated", "local"],
        });

        // store a new identity's machine as the universe with its key
        const identity = new MachineIdentity(
            Identifier.create("machine", this.environment),
            new MemoryKeychain(),
            this.environment,
        );
        const { publicKey } = await identity.generate();
        await this.store.mutate(this.store.as(machine), {
            id: RequestId.create(this.store.environment),
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
        this.#machines.set(identity.machineId, identity);

        return identity;
    }

    /** Act as a machine a test enrolled outside the fixture, by the identity holding its key. */
    register(identity: MachineIdentity): void {
        this.#machines.set(identity.machineId, identity);
    }

    /** Call as a registered machine, a space or an installation, signed by its key. */
    as(caller: MachineIdentity | string | Subject): TestCaller {
        return this.served.as(caller instanceof MachineIdentity ? caller.machineId : caller);
    }

    /** Stop serving the packages. */
    async close(): Promise<void> {
        await this.served[Symbol.asyncDispose]();
    }

    /** Open a fixture over a storage the account package serves with some keys. */
    static async #over(
        storage: TestDatabase,
        served: Served,
        keys: AccountKeys,
        options: Pick<ServeOptions, "homes" | "simulation"> & {
            /** The sessions of the people with an authenticator, by email. */
            readonly sessions: Readonly<Record<string, TestSession>>;
        },
    ): Promise<TestAccountStore> {
        return new TestAccountStore(storage, served, {
            platform: await TestAccountStore.#platform(storage.database),
            homes: options.homes,
            simulation: options.simulation,
            cloudKeychains: keys.cloud,
            keychain: keys.universe,
            sessions: options.sessions,
        });
    }

    /** Store the residencies, the people, the platform, its homes and the owner's accounts. */
    static async #write(
        storage: TestDatabase,
        options: ServeOptions,
    ): Promise<{
        readonly keys: AccountKeys;
        readonly sessions: Readonly<Record<string, TestSession>>;
    }> {
        // serve the account package with new keys, storing the platform before the universe's identity
        const keys = AccountKeys.create();
        const definition = await TestAccountStore.#platformDefinition(
            keys.cloud,
            options.simulation.environment(WORLD_NODE),
        );
        const served = await TestAccountStore.#serve(storage, keys, options, definition);
        const fixture = await TestAccountStore.#over(storage, served, keys, {
            ...options,
            sessions: {},
        });

        // sign the people up, add the operator, start the homes' identities and store the owner's accounts
        await fixture.#signUpPeople();
        await fixture.#joinOperator();
        if (options.homes === "fixture") {
            await fixture.#serveHomes();
        }
        await fixture.#createAccounts();

        // enroll the authenticators of the people who step up, then stop serving
        const sessions = await fixture.#enrollPeople();
        await fixture.close();

        return { keys, sessions };
    }

    /** Sign up the fixture's people, as sign-up stores them. */
    async #signUpPeople(): Promise<void> {
        const context = await this.authenticator.auth.$context;
        for (const { isEnrolled: _, ...person } of PEOPLE) {
            // seed each person proven, as no mailbox answers the template's write
            await context.internalAdapter.createUser(
                { ...person, emailVerified: true, ...SIGN_UP, residencyId: "eu" },
                { method: "magic-link" },
            );
        }
    }

    /** Sign the people who step up in on a client each, and enroll their authenticators. */
    async #enrollPeople(): Promise<Record<string, TestSession>> {
        // sign each in with a mailed code first, which no longer signs them in once enrolled
        const sessions: Record<string, TestSession> = {};
        for (const { email } of PEOPLE.filter((person) => person.isEnrolled)) {
            const person = await this.signIn(email);
            const authenticator = await this.#enroll(person, this.browser());
            sessions[email] = {
                userId: person.id,
                handle: person.handle,
                clientId: person.clientId,
                refreshToken: (await person.tokens()).refreshToken,
                authenticator,
            };
        }

        return sessions;
    }

    /** Declare the platform with a cloud machine per residency. */
    static async #platformDefinition(
        cloudKeychains: Map<Identifier<"machine">, MemoryKeychain>,
        environment: Environment,
    ): Promise<PlatformDefinition> {
        return {
            name: "Destack",
            residencyId: "eu",
            operator: OPERATOR_EMAIL,
            reserved: [],
            publishers: [],
            handle: PLATFORM_HANDLE,
            machines: [
                await cloudMachine(cloudKeychains, environment, ids.machine, "eu-central", "eu"),
                await cloudMachine(cloudKeychains, environment, ids.usMachine, "us-east", "us"),
            ],
        };
    }

    /** Serve the account and setting packages over a database with some keys. */
    static async #serve(
        storage: TestDatabase,
        keys: AccountKeys,
        options: ServeOptions,
        platform?: PlatformDefinition,
    ): Promise<Served> {
        // compose the universe's store of the account and setting packages, signing in once it exists
        const { database } = storage;
        const { clock } = options.simulation;
        const environment = options.simulation.environment(WORLD_NODE);
        const releases = new Map<string, Promise<BuildReader>>();
        const signIn = Promise.withResolvers<BetterAuthAuthenticator>();
        const events = await testEvents(database, environment);
        const opening = options.openRelease ?? TestAccountStore.#opening(releases);
        const base = (handle: (typeof UNIVERSE_PACKAGES)[number]) =>
            Installation.base(Scope.universe, handle, environment.crypto);
        const store = new ObjectStore({
            servers: [
                objectServer.at(await base(objectPackage)),
                accountServer({
                    resolver: options.resolver ?? new TestDns(),
                    stacks: options.stacks,
                    signIn: signIn.promise,
                    releases: { open: opening },
                }).at(await base(accountPackage)),
                settingServer({
                    declared: Setting.declared((value) =>
                        opening(
                            value.packageId,
                            present(value.release, `the release of ${value.name}`),
                        ),
                    ),
                    after: [],
                }).at(await base(settingPackage)),
            ],
            chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
            principal: Scope.universe,
            partitions: [Scope.universe.id],
            policies: options.policies ?? [],
            database,
            claims: { object: claim },
            environment,
            callKey: testCallKey,
            events,
            origin: { package: Package.of(accountPackage) },
        });

        // receive the universe with its base installations and store the platform before its identity
        if (platform !== undefined) {
            await Authorization.run(
                store.authorizer,
                store.database,
                () => AccessContext.of(Caller.of(Scope.universe), clock.now()),
                store.environment,
                (authorization) =>
                    Installation.receive(
                        authorization,
                        Scope.universe,
                        [],
                        UNIVERSE_PACKAGES,
                        store,
                    ),
            );
            await Residency.apply(store, RESIDENCIES);
            await new Platform(store, platform, options.stacks.residency).apply(clock.now());
        }

        // sign in with the universe's identity, mailing links and codes to the test mail
        const mail = new TestMail();
        const authenticate = async (keyring: Keyring) =>
            BetterAuthAuthenticator.create({
                keystore: await IdentityKeystore.universe(
                    keyring,
                    database,
                    IdentityRegistry.of(store),
                    environment,
                ),
                database,
                callKey: testCallKey,
                issuer: ISSUER,
                trustedOrigins: [ORIGIN],
                ipAddress: { disableIpTracking: true },
                providers: {},
                objects: store,
                events,
                lifetime: new TokenLifetime(new Map()),
                mail,
                ...options.signIn,
            });
        const authenticator = await authenticate(
            await LocalKeyring.open(keys.universe, "account", environment),
        );
        signIn.resolve(authenticator);

        // act as the declared cloud machines with their keys
        const machines = new Map(
            [...keys.cloud].map(([id, keychain]) => [
                id,
                new MachineIdentity(id, keychain, environment),
            ]),
        );

        // serve each package to bearers, signing machines by their keys and spaces by their space's
        const fetch = (request: Request) => TestAccountStore.#route(store, served, request);
        const issuer = new Issuer(ISSUER, fetch, environment);
        const sources = new Map<string, TokenSource>();
        const served: TestObjectStore = TestObjectStore.serve(store, {
            credential: {
                sign: async (request, signer) => {
                    // send the token the caller's key proves
                    const token = await TestAccountStore.#token(signer, {
                        machines,
                        sources,
                        keystore: authenticator.keystore,
                        database,
                        environment,
                        send: fetch,
                    });
                    const headers = new Headers(request.headers);
                    headers.set("authorization", `Bearer ${token}`);

                    return new Request(request, { headers });
                },
                authenticate: (request, audience) =>
                    AccountAuthentication.bearer(request, {
                        database,
                        issuer: ISSUER,
                        audience,
                        now: clock.now(),
                        environment,
                    }),
            },
        });
        await served.ready();

        return {
            store,
            authenticator,
            authenticate,
            served,
            machines,
            releases,
            mail,
            issuer,
            fetch,
        };
    }

    /** Route a request of the universe's origin to the served package mounted at its path. */
    static #route(
        store: ObjectStore,
        served: TestObjectStore,
        request: Request,
    ): Promise<Response> {
        // find the mounted package, refusing a path of none served
        const mount = PackageMount.of(request);
        const handler = store.packages.find(
            (implementation) => implementation.package.id === mount?.packageId,
        );
        if (mount === undefined || handler === undefined) {
            return Promise.resolve(new Response(null, { status: 404 }));
        }

        return served.server(handler.package).fetch(mount.request);
    }

    /** Read a machine's, space's or installation's token for a package, signed by its key. */
    static async #token(
        signer: TestSigner,
        keys: {
            /** The machines tests act as, by machine. */
            readonly machines: ReadonlyMap<string, MachineIdentity>;
            /** The token sources of the spaces and installations tests act as, by subject. */
            readonly sources: Map<string, TokenSource>;
            /** The keystore storing the spaces' keys. */
            readonly keystore: IdentityKeystore;
            /** The account database. */
            readonly database: DatabaseConnection;
            /** The world the calls run in. */
            readonly environment: Environment;
            /** Send a request to the universe's packages. */
            readonly send: (request: Request) => Promise<Response>;
        },
    ): Promise<string> {
        // refuse a second factor, which keys do not prove
        const { subject, audience, isSteppedUp } = signer;
        if (isSteppedUp) {
            throw new TypeError(`${subject.id} signs with a key, which proves no second factor`);
        }

        // sign a machine with its key
        if (principal.machine.is(subject)) {
            const identity = found(keys.machines, subject.id);

            return identity.token(audience, { url: ISSUER }, keys.send);
        }

        // sign a space or installation with its space's key, keeping its granted tokens
        const key = Subject.key(subject);
        const source =
            keys.sources.get(key) ??
            keys.keystore.source(keys.database, subject, {
                environment: keys.environment,
                issuer: ISSUER,
                send: keys.send,
            });
        keys.sources.set(key, source);

        return source.grantedToken(audience);
    }

    /** Open the releases a test publishes, by package and version, refusing any other. */
    static #opening(
        releases: ReadonlyMap<string, Promise<BuildReader>>,
    ): (packageId: PackageId, version: string) => Promise<BuildReader> {
        return async (packageId, version) => {
            const reader = releases.get(`${packageId}@${version}`);
            if (reader === undefined) {
                throw CallError.http("NOT_FOUND", {
                    message: `release ${version} of ${packageId} not found`,
                });
            }

            return reader;
        };
    }

    /** Read the platform's account, which the seed stores. */
    static async #platform(database: DatabaseConnection): Promise<Identifier<"account">> {
        const [row] = await database
            .select({ id: account.table.id })
            .from(account.table)
            .where(eq(account.table.handle, PLATFORM_HANDLE));

        return present(row, "the platform's account").id;
    }

    /** Place a space in an account of a person on a machine the person enrolls in it. */
    async #place(
        person: TestPerson,
        target: { readonly id: Identifier<"account">; readonly handle: string },
        spaceId: Identifier<"space">,
    ) {
        // enroll a machine, then create the space in the account, placed on it as creation places it
        const enrolled = await this.enroll(target.id, person);
        await person.client(accountPackage).space.create({
            accountId: target.id,
            id: spaceId,
            requestId: RequestId.create(this.store.environment),
            name: `space-${spaceId.slice(-12)}`,
            machine: enrolled.machineId,
        });

        return {
            accountId: target.id,
            handle: target.handle,
            spaceId,
            machine: this.as(enrolled),
        };
    }

    /** Create the owner's eu accounts as the owner. */
    async #createAccounts(): Promise<void> {
        const owner = (await this.signIn(OWNER_EMAIL)).client(accountPackage);
        for (const declared of ACCOUNTS) {
            await owner.account.create({
                scope: ids.owner,
                id: declared.id,
                requestId: RequestId.create(this.store.environment),
                handle: declared.handle,
                name: declared.handle,
                kind: declared.kind,
                residencyId: "eu",
            });
        }
    }

    /** Add the operator as the platform organization's owner through the invitation it stores. */
    async #joinOperator(): Promise<void> {
        // sign the operator in and find the platform organization
        const operator = await this.signIn(OPERATOR_EMAIL);
        const organizationId = present(
            await UniverseRole.operating(Snapshot.live(this.database)),
            "the platform organization",
        );

        // accept the invitation the platform stores for the operator
        const [invited] = (await operator.client(accountPackage).user.invited({ id: ids.operator }))
            .items;
        await operator.client(objectPackage).invitation.accept({
            scope: Scope.universe.id,
            object: organization.reference(Scope.universe.id, organizationId),
            invitationId: present(invited, "the invitation to operate").id,
            requestId: RequestId.create(this.store.environment),
        });
    }

    /** Start the identities of the residencies' homes as the cloud machines serving them. */
    async #serveHomes(): Promise<void> {
        for (const entry of await Residency.list(this.database)) {
            const home = await this.residencySpace(entry.id);
            await this.startIdentity(home, this.as(await this.machine(home)));
        }
    }

    /** Read where people sign in: the issuer, its mail, the origin, the world and the residency. */
    #signIn(): TestSignIn {
        return {
            issuer: this.issuer,
            mail: this.#mail,
            origin: ORIGIN,
            fetch: (request) => this.fetch(request),
            environment: this.environment,
            residencyId: "eu",
        };
    }

    /** Open the simulated world a fixture runs in, paused at its start. */
    static #world(): Simulation {
        return new Simulation({ seed: WORLD_SEED, start: WORLD_START });
    }

    /** Stop serving the packages, then close the database. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
        await this.storage.close();
    }
}

/** How a test enrolls a machine: its name and the residency a cloud machine serves. */
export interface MachineEnrollment {
    /** The proposed name. */
    readonly name?: string;
    /** The residency a cloud machine serves. */
    readonly residencyId?: ResidencyCode;
}

/** A client a test authorizes with PKCE: its redirect, scopes, state and package. */
export interface TestAuthorization {
    /** The client. */
    readonly clientId: string;
    /** The page the client receives its code at. */
    readonly redirectUri: string;
    /** The scopes it asks for, space separated. */
    readonly scope: string;
    /** The state it gets back with its code. */
    readonly state: string;
    /** The package its tokens are for as a resource, none for a registered client's own tokens. */
    readonly resource: string | null;
}

/** The email sign-in mailed, stored as the platform's account would send it. */
export class TestMail implements MailTransport {
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

/** Declare a cloud machine of the universe serving a residency, storing its keychain. */
async function cloudMachine(
    keychains: Map<Identifier<"machine">, MemoryKeychain>,
    environment: Environment,
    id: Identifier<"machine">,
    name: string,
    residencyId: ResidencyCode,
): Promise<PlatformDefinition["machines"][number]> {
    // generate its key, storing its keychain for tests to act as it and run it
    const keychain = new MemoryKeychain();
    const { publicKey } = await new MachineIdentity(id, keychain, environment).generate();
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
    /** The keychains' secrets, as an account template's value stores them. */
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

    /** Open keychains with the secrets of a template's value. */
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

    /** Read the keychains' secrets as a template's value stores them. */
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

    /** Open a keychain with some secrets. */
    static #keychain(secrets: Readonly<Record<string, string>>): MemoryKeychain {
        const keychain = new MemoryKeychain();
        for (const [name, secret] of Object.entries(secrets)) {
            keychain.secrets.set(name, secret);
        }

        return keychain;
    }
}

/** The one database an account template writes: the universe's. */
type AccountTemplateDatabases = { readonly universe: typeof accountDatabase };

/** The keychains' secrets, as an account template's value stores them. */
type AccountKeysValue = schema.Infer<typeof AccountKeys.schema>;

/** The account fixture's rows with the default stacks and the fixture's homes, which tests copy. */
export const ACCOUNT_TEMPLATE = TestAccountStore.template({
    name: "account",
    world: { value: schema.null(), write: async () => null },
});

import { UniverseOrigin } from "@destack/account/address";
import { Bookmark } from "@destack/object/client";
import { DEVICE_SCOPES, Issuer, MachineKey } from "@destack/account/client";
import { MemoryBlobStorage } from "@destack/resource";
import accountPackage from "@destack/account/package";
import { sudo } from "@destack/account/authentication";
import {
    authenticatorCode,
    Enrollment,
    ENTITLED_PACKAGE,
    TestBrowser,
    TestMail,
    OPERATOR_EMAIL,
    PersonSignIn,
    REGION_STACK,
    STACKS,
    TestPerson,
    PLATFORM_HANDLE,
} from "@destack/account/test";
import { MemoryBucket } from "@destack/bucket/test";
import { TestDatabase } from "@destack/db/test";
import type { Clock, Environment } from "@destack/environment";
import { testSimulation } from "@destack/object/test";
import type { Simulation } from "@destack/environment/simulation";
import { LocalKeyring, MemoryKeychain, PublicKey } from "@destack/identity";
import {
    Account,
    type Allowance,
    Entitlement,
    entitlement,
    account,
    type EntitlementResource,
    type RegionCode,
    machine,
    REGION_MERCHANT,
    space,
    type StackRelease,
} from "@destack/account/object";
import { AccessContext, Caller, InstallationId, principal } from "@destack/access";
import { CallKey, type FeatureReference, ObjectType, type ProcedureRouter } from "@destack/object";
import objectPackage from "@destack/object/package";
import type { Example } from "@destack/package/declare";
import { Identifier, present, schema } from "@destack/schema";
import { RequestId, Scope, type Subject } from "@destack/sync";
import { Platform } from "@destack/account/server";
import type { UniverseDefinition } from "../universe/definition.ts";
import { BuildLayer } from "@destack/package/manifest";
import { Universe, type UniverseOptions, UNIVERSE_PACKAGES } from "../universe/universe.ts";
import { TestMachine } from "./machine.ts";

/** The origin a test universe answers at, which no request leaves the process for. */
const TEST_ORIGIN = "https://universe.test";

/** The address of a test universe: its origin, and Destack's domains. */
export const TEST_UNIVERSE = new UniverseOrigin(TEST_ORIGIN, {
    space: "destack.space",
    machine: "destack.computer",
    site: "destack.sh",
});

/** The port the universe answers its HTTPS origin at on a simulated network. */
const HTTPS_PORT = 443;

/** The node the universe answers on in a simulated network. */
export const UNIVERSE_NODE = "universe";

/** The node people's clients and browsers send from in the simulated network. */
const CLIENT_NODE = "client";

/** The region a test universe's people live in by default. */
const TEST_REGION = "eu";

/** The loopback redirect a stepped-up native client returns to, which the step-up stops at. */
const STEP_UP_REDIRECT = "http://127.0.0.1:53682/callback";

/** The language the universe's browsers ask for. */
const BROWSER_LANGUAGE = "en";

/** The kind and hardware of the self-hosted machines a test universe enrolls. */
const SELF_MACHINE = {
    kind: "self",
    platform: "macOS",
    architecture: "arm64",
    model: "Mac14,2",
} as const;

/** What a machine declares as it enrolls: self-hosted, or a managed machine of a region. */
export type MachineDeclaration = Parameters<MachineKey["enroll"]>[1] extends infer Enrolled
    ? Enrolled extends unknown
        ? Omit<Enrolled, "accountId" | "requestId" | "name">
        : never
    : never;

/** Where an account's managed machine runs: its region, provider and location. */
export interface CloudDeclaration {
    /** The region the machine serves. */
    readonly regionId: RegionCode;
    /** The provider running it. */
    readonly providerCode: string;
    /** The provider location. */
    readonly location: string;
}

/** How long a self-hosted machine's tunnel stands before its token lapses: an hour. */
const TUNNEL_MILLISECONDS = 3_600_000;

/** The cloud provider and location a test universe's machines declare. */
const TEST_LOCATION = { providerCode: "test", location: "test-1" } as const;

/** What a test universe declares and runs at. */
export interface TestUniverseOptions {
    /** The universe's regions, Europe alone by default, each served by one managed machine. */
    readonly regions?: Readonly<UniverseDefinition["regions"]>;
    /** The handles the platform publishes first-party packages under, none by default. */
    readonly publishers?: UniverseDefinition["platform"]["publishers"];
    /** The simulation it answers on at its origin, a world of its own by default. */
    readonly simulation?: Simulation;
    /** The releases its distribution has, none by default. */
    readonly distribution?: UniverseOptions["distribution"];
    /** The builds of the distribution's releases, none by default. */
    readonly layer?: UniverseOptions["layer"];
    /** The stacks its spaces are born with, the account test fixture's by default. */
    readonly stacks?: UniverseDefinition["platform"]["stacks"];
    /** The stack the platform's home runs in each region, the account fixture's by default. */
    readonly home?: StackRelease;
    /** The migrated database it starts over, which the caller closes, a new one by default. */
    readonly storage?: TestDatabase;
    /** The keys and sign-ins a template wrote over the given database, new ones by default. */
    readonly world?: UniverseWorld;
}

/** A test universe's state beside its database: its keys and its people's sign-ins. */
export const UniverseWorld = schema.object({
    /** The universe keyring's secret. */
    keyring: schema.string(),
    /** Each region's declared managed machine and its keychain's secrets, by region. */
    machines: schema.record(
        schema.string(),
        schema.object({
            /** The machine. */
            id: schema.identifier("machine"),
            /** The secrets of the keychain keeping its key, by name. */
            secrets: schema.record(schema.string(), schema.string()),
            /** Its declared public key. */
            key: PublicKey,
        }),
    ),
    /** The people signed in, by email. */
    people: schema.record(schema.string(), PersonSignIn),
    /** The TOTP URI of each person's enrolled authenticator, by user. */
    authenticators: schema.record(schema.string(), schema.string()),
});
/** A test universe's state beside its database: its keys and its people's sign-ins. */
export type UniverseWorld = schema.Infer<typeof UniverseWorld>;

/** A region's declared managed machine, with the keychain keeping its key. */
interface DeclaredMachine {
    /** The machine. */
    readonly id: Identifier<"machine">;
    /** The keychain keeping the machine's declared key. */
    readonly keychain: MemoryKeychain;
    /** The machine's declared public key. */
    readonly key: PublicKey;
    /** The URL the machine answers at, as declared. */
    readonly origin: string;
}

/** The universe served in process over a test database, signed in to as its native clients do. */
export class TestUniverse implements AsyncDisposable {
    /** The running universe. */
    readonly universe: Universe;
    /** The test database. */
    readonly storage: TestDatabase;
    /** The mail the universe sends. */
    readonly mail: TestMail;
    /** The clock, network and randomness it runs in. */
    readonly environment: Environment;
    /** The origin the universe answers at on its simulation's network. */
    readonly origin = TEST_ORIGIN;
    /** The simulation the universe answers on. */
    readonly simulation: Simulation;
    /** The managed machine of each region, with the keychain keeping its key. */
    readonly #machines: ReadonlyMap<RegionCode, DeclaredMachine>;
    /** The machines opened as spaces' hosts, which disposal stops first. */
    readonly #opened = new AsyncDisposableStack();
    /** The universe and its database, which disposal stops and closes after the machines. */
    readonly #disposables: AsyncDisposableStack;
    /** The issuer people sign in with, as native clients reach it. */
    readonly #issuer: Issuer;
    /** The authenticator each stepped-up person enrolled, by person. */
    readonly #authenticators: Map<string, string>;
    /** The browser each person approves sign-ins in, by person. */
    readonly #browsers = new Map<string, TestBrowser>();
    /** The universe keyring's secret. */
    readonly #keyring: string;
    /** Each person signed in, by email. */
    readonly #people: Map<string, TestPerson>;
    /** The operator's joining of the platform organization, once started. */
    #joining: Promise<void> | undefined;
    /** The object types the universe serves by name, each with its package. */
    readonly #types: ReadonlyMap<
        string,
        { readonly type: ObjectType; readonly handle: (typeof UNIVERSE_PACKAGES)[number] }
    >;

    /** The machine in front of the origin, as the platform's machine stands in production. */
    readonly #front: UniverseFront;

    /** Keep a started universe and its stand-ins. */
    private constructor(
        universe: Universe,
        storage: TestDatabase,
        mail: TestMail,
        environment: Environment,
        machines: ReadonlyMap<RegionCode, DeclaredMachine>,
        disposables: AsyncDisposableStack,
        world: Pick<UniverseWorld, "keyring" | "people" | "authenticators">,
        simulation: Simulation,
        front: UniverseFront,
    ) {
        // keep the universe and the stand-ins for mail, storage and time
        this.universe = universe;
        this.#front = front;
        this.storage = storage;
        this.mail = mail;
        this.environment = environment;
        this.#machines = machines;
        this.#disposables = disposables;
        this.#keyring = world.keyring;
        this.simulation = simulation;

        // reach the issuer over the network, and index the object types it serves
        this.#issuer = new Issuer(
            TEST_UNIVERSE.issuer,
            (request) => simulation.network.of(CLIENT_NODE).fetch(request),
            environment,
            new Bookmark(),
        );
        this.#types = new Map(
            UNIVERSE_PACKAGES.flatMap((handle) =>
                Object.values(handle.objects)
                    .filter((declared) => declared instanceof ObjectType)
                    .map((type) => [type.name, { type, handle }] as const),
            ),
        );

        // restore the people the world signed in, and the authenticators they enrolled
        this.#authenticators = new Map(Object.entries(world.authenticators));
        this.#people = new Map(
            Object.entries(world.people).map(([email, signedIn]) => [
                email,
                new TestPerson(this.#signIn(), signedIn),
            ]),
        );
    }

    /** Put a machine in front of the origin for the paths the universe leaves, until disposed. */
    serve(fetch: (request: Request) => Promise<Response>): Disposable {
        this.#front.machine = fetch;

        return {
            [Symbol.dispose]: () => {
                this.#front.machine = undefined;
            },
        };
    }

    /** Start a universe over a database of the project's target, mail and storage in memory. */
    static async open(options: TestUniverseOptions = {}): Promise<TestUniverse> {
        // take the given database or migrate one, closed again should the start fail, and keep its mail, keys and call events in memory
        await using opening = new AsyncDisposableStack();
        const simulation = options.simulation ?? TestUniverse.#simulate(opening);
        const environment = simulation.environment(UNIVERSE_NODE);
        const { clock } = environment;
        const storage =
            options.storage ??
            (await TestDatabase.create(Universe.database, { environment, isMigrated: true }));
        if (options.storage === undefined) {
            opening.defer(() => storage.close());
        }
        const secret = options.world?.keyring ?? LocalKeyring.generate(environment);
        const keyring = await LocalKeyring.read(secret, environment);
        const key = CallKey.of(keyring, environment.crypto);
        const mail = new TestMail();

        // declare one managed machine per region, keeping its keychain to serve as it, the world's keys when restored
        const regions = options.regions ?? [
            {
                code: TEST_REGION,
                name: "Europe",
                relay: `https://relay-${TEST_REGION}.machine.test`,
                countries: ["DE"],
            },
        ];
        const machines = new Map<RegionCode, DeclaredMachine>();
        const declared = await Promise.all(
            regions.map(async (declaredRegion) => {
                // generate the machine's key, or restore the one the world kept
                const restored = options.world?.machines[declaredRegion.code];
                const id = restored?.id ?? Identifier.create("machine", environment);
                const keychain = new MemoryKeychain(restored?.secrets);
                const publicKey =
                    restored?.key ??
                    (await new MachineKey(id, keychain, environment).generate()).publicKey;
                const origin = `https://${declaredRegion.code}.machine.test/`;
                machines.set(declaredRegion.code, { id, keychain, key: publicKey, origin });

                return {
                    ...TEST_LOCATION,
                    id,
                    name: `${declaredRegion.code}-cloud`,
                    regionId: declaredRegion.code,
                    key: publicKey,
                    origin,
                    rates: {},
                };
            }),
        );
        const definition: UniverseDefinition = {
            regions: [...regions],
            platform: {
                name: "Destack",
                regionId: TEST_REGION,
                operator: OPERATOR_EMAIL,
                transfers: {},
                reserved: [],
                publishers: options.publishers ?? [],
                clients: [],
                handle: PLATFORM_HANDLE,
                domains: [],
                homes: Object.fromEntries(
                    regions.map(({ code }) => [code, options.home ?? REGION_STACK]),
                ),
                hostnames: Object.fromEntries(regions.map(({ code }) => [code, []])),
                stacks: options.stacks ?? STACKS,
                machines: declared,
            },
        };

        // start the universe, refusing requests to other origins
        const universe = opening.use(
            await Universe.start({
                definition,
                origin: TEST_UNIVERSE,
                database: storage.database,
                keyring,
                callKey: () => key,
                mail,
                providers: {},
                events: new MemoryBucket(clock),
                blobs: new MemoryBlobStorage(environment.crypto),
                ipAddress: { disableIpTracking: true },
                environment,
                report: (error) => simulation.scheduler.fail(error),
                distribution: options.distribution ?? [],
                layer: options.layer ?? BuildLayer.empty(),
            }),
        );

        // serve once the run applied the definition, failing on the universe's first failure
        await Promise.race([
            Promise.all([...universe.servers.values()].map((server) => server.ready())),
            simulation.scheduler.failed,
        ]);

        // answer at the origin, handing the paths the universe leaves to the machine in front
        const front: UniverseFront = { machine: undefined };
        const listener = simulation.network.server(UNIVERSE_NODE).http({
            hostname: new URL(TEST_ORIGIN).hostname,
            port: HTTPS_PORT,
            ...universe.connections.listen((request) => {
                const isUniverse = TEST_UNIVERSE.serves(request.url) !== undefined;

                return isUniverse || front.machine === undefined
                    ? universe.fetch(request)
                    : front.machine(request);
            }),
        });
        opening.defer(() => listener.stop(true));

        // keep the keyring's secret and the people the world signed in
        const world = {
            keyring: secret,
            people: options.world?.people ?? {},
            authenticators: options.world?.authenticators ?? {},
        };

        return new TestUniverse(
            universe,
            storage,
            mail,
            environment,
            machines,
            opening.move(),
            world,
            simulation,
            front,
        );
    }

    /** Send a machine's requests to the universe from its node on the network. */
    #sender(machineId: Identifier<"machine">): (request: Request) => Promise<Response> {
        return (request) => this.simulation.network.of(machineId).fetch(request);
    }

    /** The clock the universe runs at, its environment's. */
    get clock(): Clock {
        return this.environment.clock;
    }

    /** Open a world of its own, its network reaching no other origin. */
    static #simulate(opening: AsyncDisposableStack): Simulation {
        const simulation = opening.use(testSimulation());

        return simulation;
    }

    /** Sign a person up or in with a mailed code, as a native client does, on a new client. */
    async signIn(email: string): Promise<TestPerson> {
        // sign the person in, keeping them for the world
        const person = await TestPerson.signIn(this.#signIn(), email);
        this.#people.set(email, person);

        return person;
    }

    /** Sign the operator in as an owner of the platform organization, accepting its invitation. */
    async operator(): Promise<TestPerson> {
        // reuse the operator's sign-in, and join the organization once
        const operator = this.#people.get(OPERATOR_EMAIL) ?? (await this.signIn(OPERATOR_EMAIL));
        this.#joining ??= this.#join(operator);
        await this.#joining;

        return operator;
    }

    /** Accept the operator's invitation to the platform organization, if one is open. */
    async #join(operator: TestPerson): Promise<void> {
        // find the invitation the platform stores for the operator
        const user = schema.identifier("user").parse(operator.subject.id);
        const { items } = await operator.client(accountPackage).user.invited({ id: user });

        // accept it once, joining the organization as its owner
        const [invited] = items;
        if (invited !== undefined) {
            await operator.client(objectPackage).invitation.accept({
                scope: Scope.universe.id,
                id: invited.id,
                requestId: RequestId.create(this.environment),
            });
            await this.simulation.idle();
        }
    }

    /** Restore a person the world signed in, on the client they signed in on. */
    person(email: string): TestPerson {
        return present(this.#people.get(email), `a sign-in of ${email}`);
    }

    /** Read the universe's keys and its people's sign-ins, as a template's value keeps them. */
    async world(): Promise<UniverseWorld> {
        // read each person's sign-in with their client's current tokens
        const people = await Promise.all(
            [...this.#people].map(
                async ([email, person]) => [email, await person.record()] as const,
            ),
        );

        // read each declared machine's keychain
        const machines: UniverseWorld["machines"] = {};
        for (const [code, declared] of this.#machines) {
            const secrets = Object.fromEntries(declared.keychain.secrets);
            machines[code] = { id: declared.id, secrets, key: declared.key };
        }

        return {
            keyring: this.#keyring,
            machines,
            people: Object.fromEntries(people),
            authenticators: Object.fromEntries(this.#authenticators),
        };
    }

    /** Describe where people sign in as native clients do: issuer, mail and region. */
    #signIn() {
        return {
            issuer: this.#issuer,
            mail: this.mail,
            origin: TEST_ORIGIN,
            fetch: (request: Request) => this.simulation.network.of(CLIENT_NODE).fetch(request),
            environment: this.environment,
            regionId: TEST_REGION,
        };
    }

    /** Step a person up with their authenticator in a browser on a new client, as RFC 9470 does. */
    async elevate(person: TestPerson): Promise<TestPerson> {
        // ask the issuer to sign the person in again with their second factor
        const clientId = Identifier.create("client", this.environment);
        const authorization = await this.#issuer.authorize(
            clientId,
            STEP_UP_REDIRECT,
            "step-up",
            DEVICE_SCOPES,
            this.clock.now(),
            sudo,
        );
        const next = await this.approve(authorization.url, person);

        // redeem the code for the new client's tokens
        const code = present(new URL(next).searchParams.get("code"), "the authorization code");
        const { tokens } = await this.#issuer.redeem(
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

    /** Read the home of the platform's account in a region. */
    async platformHome(code: RegionCode): Promise<Identifier<"space">> {
        // read the platform's account in the region by its handle
        const handle = Platform.handle({ handle: PLATFORM_HANDLE, regionId: TEST_REGION }, code);
        const platform = await Account.byHandle(this.storage.database.snapshot(), handle);

        return Account.home(present(platform, `the platform's account in ${code}`).id);
    }

    /** Read the platform's merchant, the finance installation its home declares. */
    async #merchant(): Promise<Subject> {
        // derive the installation the platform home's stack declares under the merchant's name
        const snapshot = this.storage.database.snapshot();
        const home = await Scope.object(snapshot, await this.platformHome(TEST_REGION));
        const crypto = this.universe.store.environment.crypto;
        const id = await InstallationId.of(home, REGION_MERCHANT, crypto);

        return principal.installation.reference(home.id, id);
    }

    /** Entitle an account to a resource or feature as the platform's merchant does. */
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

        // act as the platform's merchant, as finance does
        const { store } = this.universe;
        const snapshot = this.storage.database.snapshot();
        const owner = await Account.of(snapshot, accountId);
        const provider = await this.#merchant();
        const now = this.clock.now();

        // replace the one the account has for the feature with one of the terms, keeping the others
        const stored = await snapshot
            .read(entitlement.table)
            .findMany({ where: { scope: accountId } });
        const allowance: Allowance = {
            packageId: feature.packageId,
            feature: feature.name,
            resource: feature.resource,
            value: null,
            limit: null,
            resetAt: null,
            state: null,
            ...terms,
        };
        const kept = stored.filter((row) => Entitlement.key(row) !== Entitlement.key(allowance));
        await store.mutate(AccessContext.of(Caller.of(provider), now), {
            id: RequestId.create(store.environment),
            calls: [
                account.call("entitle", {
                    ...account.target(owner),
                    allowances: [...kept.map((row) => Entitlement.allowance(row)), allowance],
                }),
            ],
        });
    }

    /** Move a space to another stack as the platform does, which its machine then applies. */
    async upgrade(
        target: { readonly accountId: Identifier<"account">; readonly id: Identifier<"space"> },
        stack: StackRelease,
    ): Promise<void> {
        const { store } = this.universe;
        const now = this.clock.now();
        await store.mutate(store.as(space, now), {
            id: RequestId.create(store.environment),
            calls: [space.call("upgrade", { ...target, stack })],
        });
    }

    /** Approve a sign-in at the issuer as a person stepped up, answering where it sends back. */
    async approve(url: string, person: TestPerson): Promise<string> {
        // enroll an authenticator the first time, then use the person's own browser, which keeps its sign-in
        const enrolled =
            this.#authenticators.get(person.id) ?? (await this.#enroll(person, this.#browser()));
        const browser = this.#browsers.get(person.id) ?? this.#browser();
        this.#browsers.set(person.id, browser);

        // follow the issuer back to the client, signing in by the mailed and authenticator's codes where asked
        const next = await browser.authorize(
            url,
            {
                email: person.email,
                code: () => this.#code(),
                authenticator: () => authenticatorCode(enrolled, this.environment),
            },
            (page) => page.origin !== TEST_ORIGIN,
        );

        return next;
    }

    /** Enroll a person's authenticator in a browser their code signs in, returning its URI. */
    async #enroll(person: TestPerson, browser: TestBrowser): Promise<string> {
        // sign the browser in by the person's code
        await browser.signIn(person.email, () => this.#code());

        // enable an authenticator and verify the code it showed a step ago
        const enabled = Enrollment.parse(
            await (await browser.fetch("/two-factor/enable", {})).json(),
        );
        await browser.fetch("/two-factor/verify-totp", {
            code: await authenticatorCode(enabled.totpURI, this.environment, -1),
        });
        this.#authenticators.set(person.id, enabled.totpURI);

        return enabled.totpURI;
    }

    /** Open a browser on the issuer's pages, reaching the universe over the network. */
    #browser(): TestBrowser {
        return new TestBrowser(
            TEST_UNIVERSE.issuer,
            (request) => this.simulation.network.of(CLIENT_NODE).fetch(request),
            BROWSER_LANGUAGE,
        );
    }

    /** Read the last code the universe mailed. */
    async #code(): Promise<string> {
        const codes = await this.mail.codes();

        return present(codes.at(-1), "a mailed code").otp;
    }

    /** Make an example's calls in the person's account as the person, each through their client. */
    async given(example: Pick<Example, "objects">, person: TestPerson): Promise<void> {
        for (const [scope, calls] of Object.entries(example.objects)) {
            // refuse calls in a scope the universe does not serve
            if (scope !== "account") {
                throw new TypeError(
                    `the test universe serves accounts alone, not the example's '${scope}'`,
                );
            }

            // call each method as the person in their account, through the package's client
            for (const { object, method, input } of calls) {
                const served = this.#types.get(object.name);
                if (served === undefined) {
                    throw new TypeError(`the test universe serves no object type ${object.name}`);
                }
                const { type, handle } = served;
                const procedure: unknown = Reflect.get(
                    Reflect.get(person.client<ProcedureRouter>(handle), type.name),
                    method,
                );
                if (typeof procedure !== "function") {
                    throw new TypeError(`${type.name} has no method '${method}'`);
                }
                await Reflect.apply(procedure, undefined, [
                    {
                        ...type.in(person.account),
                        ...(type.method(method).mutates
                            ? { requestId: RequestId.create(this.environment) }
                            : {}),
                        ...input,
                    },
                ]);
            }
        }
    }

    /** Open a region's managed machine following the directory, from a world when restored. */
    async machine(
        code: RegionCode = TEST_REGION,
        restored: Pick<Parameters<typeof TestMachine.open>[0], "storage" | "world"> = {},
    ): Promise<TestMachine> {
        // prove the region's declared machine, calling the universe in process
        const declared = present(this.#machines.get(code), `a managed machine of ${code}`);
        const opened = await TestMachine.open({
            ...restored,
            id: declared.id,
            keychain: declared.keychain,
            origin: TEST_UNIVERSE,
            send: this.#sender(declared.id),
            environment: this.environment,
        });
        this.#opened.use(opened);

        return opened;
    }

    /** Read the URL a region's declared managed machine answers at. */
    machineOrigin(code: RegionCode = TEST_REGION): string {
        return present(this.#machines.get(code), `a managed machine of ${code}`).origin;
    }

    /** Enroll a machine of an account as a person, self-hosted by default, relayed at an origin. */
    async enroll(
        person: TestPerson,
        accountId: Identifier<"account">,
        origin: string,
        declared: MachineDeclaration = SELF_MACHINE,
    ): Promise<TestMachine> {
        // enroll the machine with a new key as the person
        const id = Identifier.create("machine", this.environment);
        const keychain = new MemoryKeychain();
        const identity = new MachineKey(id, keychain, this.environment);
        await identity.enroll(person.client(accountPackage), {
            ...declared,
            accountId,
            requestId: RequestId.create(this.environment),
            name: TestMachine.nameOf(id),
        });

        return await this.#published(id, keychain, accountId, { kind: "relayed", relay: origin });
    }

    /** Declare an account's managed machine at its origin, as the platform does. */
    async declare(
        accountId: Identifier<"account">,
        origin: string,
        declared: CloudDeclaration = { regionId: TEST_REGION, ...TEST_LOCATION },
    ): Promise<TestMachine> {
        // declare the machine with a new key at the universe, as the universe trusting the platform
        const id = Identifier.create("machine", this.environment);
        const keychain = new MemoryKeychain();
        const { publicKey } = await new MachineKey(id, keychain, this.environment).generate();
        const { store } = this.universe;
        const now = this.clock.now();
        await store.mutate(store.as(machine, now), {
            id: RequestId.create(store.environment),
            calls: [
                machine.call("declare", {
                    ...machine.target({ scope: accountId, id }),
                    name: TestMachine.nameOf(id),
                    kind: "managed",
                    ...declared,
                    publicKey,
                    origin,
                }),
            ],
        });

        return await this.#published(id, keychain, accountId, { kind: "declared" });
    }

    /** Open a machine following the directory as it, published where it answers. */
    async #published(
        id: Identifier<"machine">,
        keychain: MemoryKeychain,
        accountId: Identifier<"account">,
        published:
            | { readonly kind: "declared" }
            | { readonly kind: "relayed"; readonly relay: string },
    ): Promise<TestMachine> {
        // follow the directory as the machine
        const opened = await TestMachine.open({
            id,
            keychain,
            origin: TEST_UNIVERSE,
            send: this.#sender(id),
            environment: this.environment,
        });
        this.#opened.use(opened);

        // follow a managed machine's declared endpoint into the machine's directory
        const now = this.clock.now();
        if (published.kind === "declared") {
            await opened.simulation.idle();
        }
        // record a self-hosted machine's tunnel as the machine, once its relay accepted it
        else {
            const self = principal.machine.reference(Scope.universe.id, id);
            await this.universe.store.mutate(AccessContext.of(Caller.of(self), now), {
                id: RequestId.create(this.universe.store.environment),
                calls: [
                    machine.call("relay", {
                        ...machine.target({ scope: accountId, id }),
                        url: published.relay,
                        lapsesAt: now + TUNNEL_MILLISECONDS,
                    }),
                ],
            });
        }

        return opened;
    }

    /** Read the relay a region's home declares, which the universe relates to the region. */
    async relay(code: RegionCode = TEST_REGION): Promise<Subject> {
        // derive the installation the platform home's stack declares under the relay's name
        const snapshot = this.storage.database.snapshot();
        const home = await Scope.object(snapshot, await this.platformHome(code));
        const id = await InstallationId.of(home, "relay", this.environment.crypto);

        return principal.installation.reference(home.id, id);
    }

    /** Stop the machines, then the universe, then remove its database. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#opened.disposeAsync();
        await this.#disposables.disposeAsync();
    }
}

/** The machine in front of a test universe's origin, absent until a platform machine starts. */
interface UniverseFront {
    /** Serve a request the universe does not. */
    machine: ((request: Request) => Promise<Response>) | undefined;
}

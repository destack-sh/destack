import { Issuer } from "@destack/account/native";
import accountPackage, { OAUTH_PATH } from "@destack/account/package";
import { sudo } from "@destack/account/authentication";
import {
    authenticatorCode,
    Enrollment,
    ENTITLED_PACKAGE,
    TestBrowser,
    TestMail,
    OPERATOR_EMAIL,
    PersonSignIn,
    STACKS,
    TestPerson,
} from "@destack/account/test";
import { MemoryBucket } from "@destack/bucket/test";
import { TestDatabase } from "@destack/db/test";
import type { Clock, Environment } from "@destack/environment";
import { testSimulation } from "@destack/object/test";
import type { Simulation } from "@destack/environment/simulation";
import { LocalKeyring, MemoryKeychain, PublicKey } from "@destack/identity";
import { MachineIdentity } from "@destack/account/client";
import {
    Account,
    Entitlement,
    entitlement,
    account,
    type EntitlementResource,
    type ResidencyCode,
    machine,
    organization,
    RESIDENCY_RELAY,
    Residency,
    space,
    type StackSource,
} from "@destack/account/object";
import { AccessContext, Caller, InstallationId, principal } from "@destack/access";
import { Snapshot } from "@destack/db";
import { CallKey, type FeatureReference, ObjectType, PackageMount } from "@destack/object";
import objectPackage from "@destack/object/package";
import type { Example } from "@destack/package/declare";
import { Identifier, present, schema } from "@destack/schema";
import { RequestId, Scope, type Subject } from "@destack/sync";
import type { UniverseDefinition } from "../universe/definition.ts";
import { type ReleaseRegistry, UniverseRole } from "@destack/account/server";
import {
    Universe,
    UNIVERSE_PACKAGES,
    universeDatabase,
    type UniverseOptions,
} from "../universe/universe.ts";
import { TestMachine } from "./machine.ts";

/** The origin a test universe answers at, which no request leaves the process for. */
const TEST_ORIGIN = "https://universe.test";

/** The port the universe answers its HTTPS origin at on a simulated network. */
const HTTPS_PORT = 443;

/** The node the universe answers on in a simulated network. */
export const UNIVERSE_NODE = "universe";

/** The node people's clients and browsers send from in the simulated network. */
const CLIENT_NODE = "client";

/** The residency a test universe's people live in by default. */
const TEST_RESIDENCY = "eu";

/** The loopback redirect a stepped-up native client returns to, which the step-up stops at. */
const STEP_UP_REDIRECT = "http://127.0.0.1:53682/callback";

/** The redirects a browser follows at most, the Fetch standard's limit. */
const FETCH_REDIRECTS = 20;

/** The button a person presses on each page a step-up passes. */
const STEP_UP_ACTIONS = [["/consent", "approve"]] as const;

/** The language the universe's browsers ask for. */
const BROWSER_LANGUAGE = "en";

/** The kind and hardware of the personal machines a test universe enrolls. */
const PERSONAL_MACHINE = {
    kind: "personal",
    platform: "macOS",
    architecture: "arm64",
    model: "Mac14,2",
} as const;

/** What a machine declares of itself as it enrolls: personal, or a cloud machine of a residency. */
export type MachineDeclaration = Parameters<MachineIdentity["enroll"]>[1] extends infer Enrolled
    ? Enrolled extends unknown
        ? Omit<Enrolled, "scope" | "requestId" | "name">
        : never
    : never;

/** Where an account's dedicated cloud machine runs: its residency, provider and location. */
export interface CloudDeclaration {
    /** The residency the machine serves. */
    readonly residencyId: ResidencyCode;
    /** The provider running it. */
    readonly providerCode: string;
    /** The provider location. */
    readonly location: string;
}

/** How long a personal machine's tunnel stands before its token lapses: an hour, past any test. */
const TUNNEL_MILLISECONDS = 3_600_000;

/** The cloud provider and location a test universe's machines declare. */
const TEST_LOCATION = { providerCode: "test", location: "test-1" } as const;

/** What a test universe declares and runs at. */
export interface TestUniverseOptions {
    /** The universe's residencies, Europe alone by default, each served by one cloud machine. */
    readonly residencies?: readonly { readonly code: ResidencyCode; readonly name: string }[];
    /** The simulation it answers on at its origin, a world of its own by default. */
    readonly simulation?: Simulation;
    /** The registry it opens packages' releases from, refusing every release by default. */
    readonly releases?: ReleaseRegistry;
    /** The stacks its spaces are born with, the account test kit's by default. */
    readonly stacks?: UniverseDefinition["stacks"];
    /** The policies of other packages it decides access for, none by default. */
    readonly policies?: UniverseOptions["policies"];
    /** The access token lifetimes longer than a minute, by audience, none by default. */
    readonly lifetimes?: UniverseOptions["lifetimes"];
    /** The migrated database it starts over, which the caller closes, a new one by default. */
    readonly storage?: TestDatabase;
    /** The keys and sign-ins a template wrote over the given database, new ones by default. */
    readonly world?: UniverseWorld;
}

/** A test universe's state beside its database: its keys and its people's sign-ins. */
export const UniverseWorld = schema.object({
    /** The universe keyring's secret. */
    keyring: schema.string(),
    /** Each residency's declared cloud machine and its keychain's secrets, by residency. */
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
});
/** A test universe's state beside its database: its keys and its people's sign-ins. */
export type UniverseWorld = schema.Infer<typeof UniverseWorld>;

/** A residency's declared cloud machine, with the keychain keeping its key. */
interface DeclaredMachine {
    /** The machine. */
    readonly id: Identifier<"machine">;
    /** The keychain keeping the machine's declared key. */
    readonly keychain: MemoryKeychain;
    /** The machine's declared public key. */
    readonly key: PublicKey;
}

/** The universe served in process over a test database, signed in to as its native clients do. */
export class TestUniverse implements AsyncDisposable {
    /** The universe. */
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
    readonly #simulation: Simulation;
    /** The cloud machine of each residency, with the keychain keeping its key. */
    readonly #machines: ReadonlyMap<ResidencyCode, DeclaredMachine>;
    /** The machines opened as spaces' hosts, which disposal stops first. */
    readonly #opened = new AsyncDisposableStack();
    /** The universe and its database, which disposal stops and closes after the machines. */
    readonly #disposables: AsyncDisposableStack;
    /** The issuer people sign in with, as native clients reach it. */
    readonly #issuer: Issuer;
    /** The authenticator each stepped-up person enrolled, by person. */
    readonly #authenticators = new Map<string, string>();
    /** The failures the universe reported to no one, which settling throws. */
    readonly #failures: unknown[];
    /** The universe keyring's secret. */
    readonly #keyring: string;
    /** Each person signed in, by email. */
    readonly #people: Map<string, TestPerson>;
    /** The object types the universe serves by name, each with its package. */
    readonly #types: ReadonlyMap<
        string,
        { readonly type: ObjectType; readonly handle: (typeof UNIVERSE_PACKAGES)[number] }
    >;

    /** Keep a started universe and its adapters. */
    private constructor(
        universe: Universe,
        storage: TestDatabase,
        mail: TestMail,
        environment: Environment,
        machines: ReadonlyMap<ResidencyCode, DeclaredMachine>,
        failures: unknown[],
        disposables: AsyncDisposableStack,
        world: Pick<UniverseWorld, "keyring" | "people">,
        simulation: Simulation,
    ) {
        // keep the universe and the adapters standing in for mail, storage and time
        this.universe = universe;
        this.storage = storage;
        this.mail = mail;
        this.environment = environment;
        this.#machines = machines;
        this.#disposables = disposables;
        this.#failures = failures;
        this.#keyring = world.keyring;
        this.#simulation = simulation;

        // reach the issuer over the network, and index the object types it serves
        this.#issuer = new Issuer(
            `${PackageMount.url(TEST_ORIGIN, accountPackage.id)}${OAUTH_PATH}`,
            (request) => simulation.network.of(CLIENT_NODE).fetch(request),
            environment,
        );
        this.#types = new Map(
            UNIVERSE_PACKAGES.flatMap((handle) =>
                Object.values(handle.objects)
                    .filter((declared) => declared instanceof ObjectType)
                    .map((type) => [type.name, { type, handle }] as const),
            ),
        );

        // restore the people the world signed in
        this.#people = new Map(
            Object.entries(world.people).map(([email, signedIn]) => [
                email,
                new TestPerson(this.#signIn(), signedIn),
            ]),
        );
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
            (await TestDatabase.create(universeDatabase, { environment, isMigrated: true }));
        if (options.storage === undefined) {
            opening.defer(() => storage.close());
        }
        const secret = options.world?.keyring ?? LocalKeyring.generate(environment);
        const keyring = await LocalKeyring.read(secret, environment);
        const key = CallKey.of(keyring, environment.crypto);
        const mail = new TestMail();
        const failures: unknown[] = [];
        const failed = Promise.withResolvers<never>();

        // declare one cloud machine per residency, keeping its keychain to serve as it, the world's keys when restored
        const residencies = options.residencies ?? [{ code: TEST_RESIDENCY, name: "Europe" }];
        const machines = new Map<ResidencyCode, DeclaredMachine>();
        const declared = await Promise.all(
            residencies.map(async (residency) => {
                // generate the machine's key, or restore the one the world kept
                const restored = options.world?.machines[residency.code];
                const id = restored?.id ?? Identifier.create("machine", environment);
                const keychain = new MemoryKeychain(restored?.secrets);
                const publicKey =
                    restored?.key ??
                    (await new MachineIdentity(id, keychain, environment).generate()).publicKey;
                machines.set(residency.code, { id, keychain, key: publicKey });

                return {
                    ...TEST_LOCATION,
                    id,
                    name: `${residency.code}-cloud`,
                    residencyId: residency.code,
                    key: publicKey,
                    origin: `https://${residency.code}.machine.test/`,
                };
            }),
        );
        const definition: UniverseDefinition = {
            residencies: [...residencies],
            platform: {
                name: "Destack",
                residencyId: TEST_RESIDENCY,
                operator: OPERATOR_EMAIL,
                reserved: [],
                publishers: [],
                handle: "destack",
                machines: declared,
            },
            stacks: options.stacks ?? STACKS,
        };

        // start the universe, refusing requests to other origins
        const universe = opening.use(
            await Universe.start({
                definition,
                origin: TEST_ORIGIN,
                database: storage.database,
                keyring,
                callKey: () => key,
                mail,
                events: new MemoryBucket(clock),
                ipAddress: { disableIpTracking: true },
                environment,
                report: (error) => {
                    failures.push(error);
                    failed.reject(error);
                },
                releases: () =>
                    options.releases ?? {
                        open: async (packageId, release) => {
                            throw new TypeError(
                                `the test universe opens no release ${packageId} ${release}`,
                            );
                        },
                    },
                policies: options.policies ?? [],
                lifetimes: options.lifetimes ?? new Map(),
            }),
        );

        // serve once the run applied the definition, failing on the universe's first failure
        failed.promise.catch(() => undefined);
        await Promise.race([
            Promise.all([...universe.servers.values()].map((server) => server.ready())),
            failed.promise,
        ]);

        // answer at the origin on the simulation's network
        const listener = simulation.network.server(UNIVERSE_NODE).http({
            hostname: new URL(TEST_ORIGIN).hostname,
            port: HTTPS_PORT,
            fetch: (request) => universe.fetch(request),
        });
        opening.defer(() => listener.stop(true));

        // keep the keyring's secret and the people the world signed in
        const world = { keyring: secret, people: options.world?.people ?? {} };

        return new TestUniverse(
            universe,
            storage,
            mail,
            environment,
            machines,
            failures,
            opening.move(),
            world,
            simulation,
        );
    }

    /** Send a machine's requests to the universe from its node on the network. */
    #sender(machineId: Identifier<"machine">): (request: Request) => Promise<Response> {
        return (request) => this.#simulation.network.of(machineId).fetch(request);
    }

    /** The clock the universe runs at, its environment's. */
    get clock(): Clock {
        return this.environment.clock;
    }

    /** Open a world of its own whose clock flows, its network reaching no other origin. */
    static #simulate(opening: AsyncDisposableStack): Simulation {
        const simulation = opening.use(testSimulation());
        simulation.clock.resume();

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
        // sign the operator in, and find the invitation the platform stores for them
        const operator = await this.signIn(OPERATOR_EMAIL);
        const organizationId = present(
            await UniverseRole.operating(Snapshot.live(this.storage.database)),
            "the platform organization",
        );
        const user = schema.identifier("user").parse(operator.subject.id);
        const { items } = await operator.client(accountPackage).user.invited({ id: user });

        // accept it once, joining the organization as its owner
        const [invited] = items;
        if (invited !== undefined) {
            await operator.client(objectPackage).invitation.accept({
                scope: Scope.universe.id,
                object: organization.reference(Scope.universe.id, organizationId),
                invitationId: invited.id,
                requestId: RequestId.create(this.environment),
            });
            await this.settle();
        }

        return operator;
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

        return { keyring: this.#keyring, machines, people: Object.fromEntries(people) };
    }

    /** Describe where people sign in as native clients do: issuer, mail and residency. */
    #signIn() {
        return {
            issuer: this.#issuer,
            mail: this.mail,
            origin: TEST_ORIGIN,
            fetch: (request: Request) => this.#simulation.network.of(CLIENT_NODE).fetch(request),
            environment: this.environment,
            residencyId: TEST_RESIDENCY,
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

    /** Entitle an account to a resource or feature as its residency's home does. */
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
        const { store } = this.universe;
        const snapshot = Snapshot.live(this.storage.database);
        const home = await Account.locate(snapshot, accountId);
        const provider = principal.space.reference(Scope.universe.id, home);
        const now = this.clock.now();

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
        await store.mutate(AccessContext.of(Caller.of(provider), now), {
            id: RequestId.create(store.environment),
            calls: [
                ...(existing === undefined
                    ? []
                    : [entitlement.call("delete", entitlement.target(existing))]),
                created,
            ],
        });
    }

    /** Move a space to another stack as the platform does, which its machine then applies. */
    async upgrade(
        target: { readonly accountId: Identifier<"account">; readonly id: Identifier<"space"> },
        stack: StackSource,
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
        // enroll an authenticator the first time, in a browser the person's link signs in
        const browser = this.#browser();
        const enrolled =
            this.#authenticators.get(person.id) ?? (await this.#enroll(person, this.#browser()));
        const asked = new URL(TestBrowser.location(await browser.fetch(url)));

        // sign in by the mailed link, and answer the second-factor challenge with the authenticator's code
        await browser.fetch(
            asked.href,
            new URLSearchParams({ action: "link", email: person.email }),
        );
        const linked = await browser.fetch(await this.#link());
        const verified = await browser.fetch(
            TestBrowser.location(linked),
            new URLSearchParams({
                action: "totp",
                code: await authenticatorCode(enrolled, this.environment),
            }),
        );

        // follow the issuer until it sends the browser back to the client, continuing the sign-in and approving its consent
        let next = TestBrowser.location(verified);
        for (let passed = 0; new URL(next).origin === TEST_ORIGIN; passed += 1) {
            if (passed >= FETCH_REDIRECTS) {
                throw new TypeError(`the sign-in redirected more than ${FETCH_REDIRECTS} times`);
            }
            const page = new URL(next);
            const action = STEP_UP_ACTIONS.find(([path]) => page.pathname.endsWith(path))?.[1];
            const answered = await browser.fetch(
                page.href,
                action === undefined ? undefined : new URLSearchParams({ action }),
            );
            next = TestBrowser.location(answered);
        }

        return next;
    }

    /** Enroll a person's authenticator in a browser their link signs in, returning its URI. */
    async #enroll(person: TestPerson, browser: TestBrowser): Promise<string> {
        // sign the browser in by the person's link
        await browser.fetch("/sign-in/magic-link", { email: person.email, callbackURL: "/" });
        await browser.fetch(await this.#link());

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
            `${PackageMount.url(TEST_ORIGIN, accountPackage.id)}${OAUTH_PATH}`,
            (request) => this.#simulation.network.of(CLIENT_NODE).fetch(request),
            BROWSER_LANGUAGE,
        );
    }

    /** Read the last link the universe mailed. */
    async #link(): Promise<string> {
        const links = await this.mail.links();

        return present(links.at(-1), "a mailed link").url;
    }

    /** Make an example's calls in the person's account as the person, each through their client. */
    async given(example: Pick<Example, "objects">, person: TestPerson): Promise<void> {
        for (const [scope, calls] of Object.entries(example.objects)) {
            // refuse calls in a scope the universe does not serve
            if (scope !== "account") {
                throw new TypeError(
                    `the test universe serves accounts alone, not the example's ${scope}`,
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
                    Reflect.get(person.client(handle), type.name),
                    method,
                );
                if (typeof procedure !== "function") {
                    throw new TypeError(`${type.name} has no method ${method}`);
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

    /** Open a residency's cloud machine following the directory, from a world when restored. */
    async machine(
        residency: ResidencyCode = TEST_RESIDENCY,
        restored: Pick<Parameters<typeof TestMachine.open>[0], "storage" | "world"> = {},
    ): Promise<TestMachine> {
        // prove the residency's declared machine, calling the universe in process
        const declared = present(this.#machines.get(residency), `a cloud machine of ${residency}`);
        const opened = await TestMachine.open({
            ...restored,
            id: declared.id,
            keychain: declared.keychain,
            origin: TEST_ORIGIN,
            send: this.#sender(declared.id),
            environment: this.environment,
        });
        this.#opened.use(opened);

        return opened;
    }

    /** Enroll a machine of an account as a person, personal by default, relayed at an origin. */
    async enroll(
        person: TestPerson,
        accountId: Identifier<"account">,
        origin: string,
        declared: MachineDeclaration = PERSONAL_MACHINE,
    ): Promise<TestMachine> {
        // enroll the machine with a new key as the person
        const id = Identifier.create("machine", this.environment);
        const keychain = new MemoryKeychain();
        const identity = new MachineIdentity(id, keychain, this.environment);
        await identity.enroll(person.client(accountPackage), {
            ...declared,
            scope: accountId,
            requestId: RequestId.create(this.environment),
            name: TestMachine.nameOf(id),
        });

        return await this.#published(id, keychain, accountId, { relay: origin });
    }

    /** Declare an account's dedicated cloud machine as the platform does, under its plan. */
    async declare(
        accountId: Identifier<"account">,
        origin: string,
        declared: CloudDeclaration = { residencyId: TEST_RESIDENCY, ...TEST_LOCATION },
    ): Promise<TestMachine> {
        // declare the machine with a new key at the universe, as the universe trusting the platform
        const id = Identifier.create("machine", this.environment);
        const keychain = new MemoryKeychain();
        const { publicKey } = await new MachineIdentity(id, keychain, this.environment).generate();
        const { store } = this.universe;
        const now = this.clock.now();
        await store.mutate(store.as(machine, now), {
            id: RequestId.create(store.environment),
            calls: [
                machine.call("declare", {
                    ...machine.target({ scope: accountId, id }),
                    name: TestMachine.nameOf(id),
                    kind: "cloud",
                    ...declared,
                    publicKey,
                }),
            ],
        });

        return await this.#published(id, keychain, accountId, { url: origin });
    }

    /** Open a machine following the directory as it, published where it answers. */
    async #published(
        id: Identifier<"machine">,
        keychain: MemoryKeychain,
        accountId: Identifier<"account">,
        published: { readonly url: string } | { readonly relay: string },
    ): Promise<TestMachine> {
        // follow the directory as the machine
        const opened = await TestMachine.open({
            id,
            keychain,
            origin: TEST_ORIGIN,
            send: this.#sender(id),
            environment: this.environment,
        });
        this.#opened.use(opened);

        // publish where a cloud machine answers as the machine
        const now = this.clock.now();
        if ("url" in published) {
            await opened.publish(published.url);
        }
        // publish a personal machine's tunnel as its account's residency's relay accepting it
        else {
            const owning = await Snapshot.live(this.storage.database)
                .read(account.table)
                .findFirst({ where: { id: accountId } });
            const relay = await this.relay(present(owning, `account ${accountId}`).residencyId);
            await this.universe.store.mutate(AccessContext.of(Caller.of(relay), now), {
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

    /** Read the relay a residency's home declares, which the universe relates to the residency. */
    async relay(residency: ResidencyCode = TEST_RESIDENCY): Promise<Subject> {
        // derive the installation the home's stack declares under the relay's name
        const snapshot = Snapshot.live(this.storage.database);
        const home = await Scope.object(snapshot, await Residency.space(snapshot, residency));
        const id = await InstallationId.declared(home, RESIDENCY_RELAY, this.environment.crypto);

        return principal.installation.reference(home.id, id);
    }

    /** Settle once no controller of any package has work due, failing on a reported failure. */
    async settle(): Promise<void> {
        await this.universe.idle();
        const failures = this.#failures.splice(0);
        if (failures.length > 0) {
            throw new AggregateError(failures, "the test universe failed");
        }
    }

    /** Stop the machines, then the universe, then remove its database. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#opened.disposeAsync();
        await this.#disposables.disposeAsync();
    }
}

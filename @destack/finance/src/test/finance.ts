import accountPackage from "@destack/account/package";
import objectPackage from "@destack/object/package";
import { AccessContext, Caller, principal } from "@destack/access";
import { account, organization, Residency } from "@destack/account/object";
import { OPERATOR_EMAIL, type TestPerson } from "@destack/account/test";
import { reconciliation } from "@destack/db/test";
import type { Crypto, Clock, Environment } from "@destack/environment";
import type { Simulation } from "@destack/environment/simulation";
import { EventTime } from "@destack/event";
import type { EventStore } from "@destack/event/store";
import { publishBuild } from "@destack/forge/test";
import { notification } from "@destack/notification/object";
import messagePackage from "@destack/message/package";
import notificationPackage from "@destack/notification/package";
import { TestNotification } from "@destack/notification/test";
import { EventView, implementPackage, type ObjectStore } from "@destack/object/server";
import type { ServerContext } from "@destack/object/workload";
import { describeMeter } from "@destack/telemetry/inspect";
import { Meter, METER_KIND } from "@destack/telemetry/meter";
import { PackageVersion } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import type { BuildReader } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { Identifier, present, schema } from "@destack/schema";
import { type StackDocument, install } from "@destack/stack/declare";
import {
    CONNECTION,
    FORGE,
    FORGE_HOME_OPTIONS,
    FORGE_HOME_STACK,
    FORGE_HOME_TYPES,
    TestPackage,
    TestSpace,
    VAULT,
} from "@destack/machine/test";
import connectionPackage, { CONNECTION_INSTALLATION } from "@destack/connection/package";
import { connectionVault } from "@destack/connection/stack";
import { stripe } from "../declare/connection.ts";
import { secret } from "@destack/vault/object";
import vaultPackage from "@destack/vault/package";
import { TestVault } from "@destack/vault/test";
import { RequestId, Scope, Subject } from "@destack/sync";
import { TestTelemetry } from "@destack/telemetry/test";
import { testBuild } from "@destack/object/test";
import { testHost } from "@destack/test/host";
import { Feature } from "../feature/index.ts";
import { describeFeature, describeSku } from "../inspect/index.ts";
import { usage } from "@destack/stack/object";
import { type Resolution, Usage, usageLimit } from "../object/index.ts";
import financePackage, { FINANCE_INSTALLATION } from "../package.ts";
import { FEATURE_KIND } from "../feature/feature.ts";
import { SKU_KIND } from "../sku/sku.ts";
import type { PaymentProvider } from "../provider/index.ts";
import type { Period } from "../rate/index.ts";
import { CustomerEntitlement } from "../server/entitlement.ts";
import { Metering } from "../server/metering.ts";
import { implementFinance } from "../server/server.ts";
import { Sku } from "../sku/index.ts";

/** The residency whose home finance serves in. */
const RESIDENCY = "eu";

/** The tag the fixture's releases open at, which answers each package's latest. */
const RELEASE_TAG = "latest";

/** The tenancies a plan granting dedicated machines lets an account run. */
const TENANCIES = ["shared", "dedicated", "local"];

/** The origin of the dedicated machines the fixture declares. */
const DEDICATED_ORIGIN = "https://dedicated.machine.test/";

/** The people the scenarios act as, by the name of their email; dave operates the platform. */
export const PEOPLE = ["alice", "bob", "carol", "dave", "eve"] as const;

/** A person the scenarios act as. */
export type Person = (typeof PEOPLE)[number];

/** The organization and accounts the scenarios bill, the platform's apart. */
export const ids = {
    example: organization.identifier("organization-019f5530-8000-7000-8000-000000000301"),
    buyer: account.identifier("account-019f5530-8000-7000-8000-000000000302"),
    payer: account.identifier("account-019f5530-8000-7000-8000-000000000303"),
    shop: account.identifier("account-019f5530-8000-7000-8000-000000000304"),
};

/** The spaces of the buyer's account the scenarios use. */
export const spaces = {
    notes: schema.identifier("space").parse("space-019f5530-8000-7000-8000-0000000000a1"),
    docs: schema.identifier("space").parse("space-019f5530-8000-7000-8000-0000000000b2"),
};

/** A use of a resource in a space an account pays for, as its machine measured it. */
export interface FinanceUse {
    /** The use's name in its scenario, which its identifier derives from. */
    readonly name: string;
    /** The space the use ran in, absent for a dedicated machine's own hours. */
    readonly space?: Identifier<"space">;
    /** The dedicated machine whose hours were used, absent for a space's use. */
    readonly machine?: Identifier<"machine">;
    /** The meter measuring it. */
    readonly meter: DeclarationReference;
    /** The amount used in the meter's unit. */
    readonly quantity: number;
    /** The level held, for an averaged meter. */
    readonly level?: number;
    /** The SKU rating it, absent for a seller's own meter. */
    readonly sku?: DeclarationReference;
    /** When the use was measured, in UTC epoch milliseconds. */
    readonly time: number;
}

/** The options of a finance fixture. */
export interface FinanceFixtureOptions {
    /** Open the payment provider billing through for the home, none by default. */
    readonly provider?: (home: string) => PaymentProvider;
    /** The moment the test starts at, as the template's world left it by default. */
    readonly start?: number;
}

/** Finance's build, declaring its package, notifications, secrets and resources. */
const FINANCE: TestPackage = {
    handle: financePackage,
    build: await testBuild(financePackage, testHost().crypto, {
        declared: [
            {
                kind: "notification",
                kindPackageId: notification.package.id,
                name: usageLimit.name,
                description: usageLimit.describe(),
            },
            ...TestVault.declarations([], Object.values(financePackage.secrets)),
        ],
        outputs: {},
    }),
    server: financeServer(new Map(), undefined),
    connectors: [],
    providers: [],
};

/** The packages the residency's home installs beside its base ones, as its stack does. */
const PACKAGES: readonly TestPackage[] = [
    VAULT,
    FORGE,
    CONNECTION,
    ...(await TestNotification.packages(testHost().crypto)),
    FINANCE,
];

/** The packages whose types the home's stack declares. */
const HOME_TYPES = [...FORGE_HOME_TYPES, connectionPackage, financePackage] as const;

/** The home's stack: finance bound to the platform's Stripe connection and webhook secret. */
const RESIDENCY_STACK = {
    ...FORGE_HOME_STACK,
    vaults: { ...FORGE_HOME_STACK.vaults, connections: connectionVault.provision() },
    connections: {
        stripe: {
            ...stripe.provision(),
            protocol: { kind: "key", header: "authorization", scheme: "Bearer" } as const,
        },
    },
    secrets: {
        ...FORGE_HOME_STACK.secrets,
        "stripe-webhook": { parent: "connections", name: "stripe-webhook" },
    },
    installations: {
        ...FORGE_HOME_STACK.installations,
        connection: install(
            connectionPackage,
            { credentials: "connections" },
            { isolation: "host", name: CONNECTION_INSTALLATION },
        ),
        message: install(messagePackage, {}, { name: "message" }),
        notification: install(notificationPackage, {}, { name: "notification" }),
        finance: install(
            financePackage,
            { stripe: "stripe", "stripe-webhook": "stripe-webhook" },
            { name: FINANCE_INSTALLATION },
        ),
    },
};

/** The home the fixture's tests copy: forge's residency home with finance and the publisher. */
const HOME_OPTIONS = {
    ...TestPackage.options(PACKAGES),
    tables: FORGE_HOME_OPTIONS.tables,
    follows: FORGE_HOME_OPTIONS.follows,
    resources: FORGE_HOME_OPTIONS.resources,
    publishers: ["platform"],
    clients: [],
};

/** What the fixture's world shares: the platform's account. */
const FinanceWorld = schema.object({
    /** The platform's account in the residency, whose home is the residency's. */
    platform: schema.identifier("account"),
});

/** What the fixture's world shares. */
type FinanceWorld = schema.Infer<typeof FinanceWorld>;

/** Finance in the residency's home with Example's buyer and payer, and Carol's shop. */
export class TestFinance {
    /** The residency's home serving finance. */
    readonly space: TestSpace;
    /** The home's object store, as its machine serves it now. */
    #store: ObjectStore;
    /** The payment provider billing through, absent for a fixture selling nothing. */
    readonly provider: PaymentProvider | undefined;
    /** The telemetry recording the exceptions finance captures, refused at close unless taken. */
    readonly telemetry: TestTelemetry;
    /** The platform's account in the residency. */
    readonly platform: Identifier<"account">;
    /** The published builds the metering reads, by package and version. */
    readonly #published: Map<string, BuildReader>;
    /** What finance's server implements in the home, which the test builds its readers from. */
    readonly #implementation: FinanceDefinition;

    /** Keep the home serving finance and the world the template wrote. */
    private constructor(fields: {
        readonly space: TestSpace;
        readonly store: ObjectStore;
        readonly implementation: FinanceDefinition;
        readonly provider: PaymentProvider | undefined;
        readonly telemetry: TestTelemetry;
        readonly world: FinanceWorld;
        readonly published: Map<string, BuildReader>;
    }) {
        // keep the home, its metering and its payment provider
        this.space = fields.space;
        this.#store = fields.store;
        this.provider = fields.provider;
        this.telemetry = fields.telemetry;

        // keep the world's accounts and machines, and the published builds
        this.platform = fields.world.platform;
        this.#published = fields.published;
        this.#implementation = fields.implementation;
    }

    /** Serve a copy of the template's home with package releases, billing through a provider. */
    static async open(
        releases: readonly (readonly [PackageVersion, readonly (Feature | Meter | Sku)[]])[],
        options: FinanceFixtureOptions = {},
    ): Promise<TestFinance> {
        // build each release
        const published = new Map<string, BuildReader>();
        const { crypto } = testHost();
        for (const [owner, declared] of releases) {
            const build = await TestFinance.release(owner, declared, crypto);
            published.set(releaseKey(owner.id, owner.version), build);
        }

        // copy the home with finance billing through the provider and metering against the releases
        const charging = options.provider;
        const implemented = Promise.withResolvers<FinanceDefinition>();
        const server = financeServer(published, charging, (definition) =>
            implemented.resolve(definition),
        );
        const { space: home, world } = await TestSpace.copy(TestFinance.template(), {
            ...HOME_OPTIONS,
            servers: HOME_OPTIONS.servers.map((each) =>
                each.package.id === financePackage.id ? server : each,
            ),
        });

        // read the store the copy serves, start at the test's start, and publish the releases as the platform
        const store = await home.store();
        const implementation = await implemented.promise;
        if (options.start !== undefined) {
            await home.simulation.jump(options.start);
        }
        for (const build of published.values()) {
            await publishBuild(home.database, home.id, world.platform, build, home.environment);
        }

        return new TestFinance({
            space: home,
            store,
            implementation,
            provider: charging === undefined ? undefined : charging(home.id),
            telemetry: TestTelemetry.of(home.simulation.telemetry),
            world,
            published,
        });
    }

    /** Define the template of the fixture's world, which every test copies. */
    static template() {
        return TestSpace.template(HOME_OPTIONS, {
            name: "finance",
            value: FinanceWorld,
            spaces: [{ residency: RESIDENCY }],
            write: (home) => TestFinance.#write(home),
        });
    }

    /** Install finance in the residency's home, and create Example, the shop and their spaces. */
    static async #write(home: TestSpace): Promise<FinanceWorld> {
        // install finance bound to the Stripe connection, with the vault and connections it uses, in one apply
        await home.serve();
        await home.apply(HOME_TYPES, RESIDENCY_STACK);

        // sign the people in, alice creating Example paying with its billing account, and carol her shop
        const { universe } = home;
        const people = new Map<Person, TestPerson>();
        for (const name of PEOPLE) {
            const person =
                name === "dave"
                    ? await universe.operator()
                    : await universe.signIn(TestFinance.#email(name));
            people.set(name, person);
        }
        const alice = present(people.get("alice"), "alice");
        const carol = present(people.get("carol"), "carol");
        const bob = present(people.get("bob"), "bob");
        const accounts = alice.client(accountPackage);
        await accounts.organization.create({
            id: ids.example,
            requestId: RequestId.create(home.environment),
            name: "Example",
            kind: "business",
            handle: "example-group",
            residencyId: RESIDENCY,
        });
        for (const [id, handle] of [
            [ids.buyer, "example-buyer"],
            [ids.payer, "example-billing"],
        ] as const) {
            await accounts.account.create({
                scope: ids.example,
                id,
                requestId: RequestId.create(home.environment),
                handle,
                name: handle,
                kind: "shared",
                residencyId: RESIDENCY,
            });
        }
        await accounts.organization.update({
            id: ids.example,
            requestId: RequestId.create(home.environment),
            payer: ids.payer,
        });
        await TestSpace.account(universe, carol, { id: ids.shop, handle: "shop" });

        // let bob view the buyer's account
        await alice.client(objectPackage).relationship.create({
            scope: ids.example,
            requestId: RequestId.create(home.environment),
            object: account.reference(ids.example, ids.buyer),
            relation: "viewer",
            subject: bob.subject,
        });

        // create the buyer's spaces
        for (const [name, id] of Object.entries(spaces)) {
            await accounts.space.create({
                accountId: ids.buyer,
                id,
                requestId: RequestId.create(home.environment),
                name,
            });
        }

        // follow them in the home, and read the platform's account there
        await home.serve();
        await home.simulation.idle();
        const platform = await home.database
            .snapshot()
            .read(account.table)
            .findFirst({ where: { handle: "destack" } });

        return { platform: present(platform, "the platform's account").id };
    }

    /** The residency's home, where finance stores its rows. */
    get home(): Identifier<"space"> {
        return this.space.id;
    }

    /** The home's object store, as its machine serves it now. */
    get store(): ObjectStore {
        return this.#store;
    }

    /** The home's database. */
    get database() {
        return this.space.database;
    }

    /** The event log recording the calls and the payers' usage. */
    get events(): EventStore {
        return this.#store.events;
    }

    /** The world the home's servers run in. */
    get environment(): Environment {
        return this.space.environment;
    }

    /** The clock finance runs at, its world's. */
    get clock(): Clock {
        return this.space.environment.clock;
    }

    /** Declare dedicated cloud machines of an account, as the platform does under its plan. */
    async declareMachines(
        accountId: Identifier<"account">,
        count: number,
    ): Promise<Identifier<"machine">[]> {
        // entitle the account to dedicated tenancy, which declaring its machines requires
        await this.space.universe.entitle(accountId, "tenancy", {
            kind: "static",
            value: TENANCIES,
        });
        const machines: Identifier<"machine">[] = [];
        for (let index = 0; index < count; index += 1) {
            machines.push((await this.space.universe.declare(accountId, DEDICATED_ORIGIN)).id);
        }

        return machines;
    }

    /** The simulation running the home's world. */
    get simulation(): Simulation {
        return this.space.simulation;
    }

    /** Read a person of the scenarios. */
    person(name: Person): TestPerson {
        return this.space.universe.person(TestFinance.#email(name));
    }

    /** Read a person's subject. */
    subject(name: Person): Subject {
        return this.person(name).subject;
    }

    /** Address a person by name, dave as the platform's operator. */
    static #email(name: Person): string {
        return name === "dave" ? OPERATOR_EMAIL : `${name}@example.test`;
    }

    /** Append an account's uses, then rate them. */
    async use(payer: string, uses: readonly FinanceUse[]): Promise<void> {
        await this.append(payer, uses);
        await this.idle();
    }

    /** Fold a paying account's usage of a meter as rating reads it, in a period or now. */
    usage(payer: string, meter: Meter, period: Period | undefined): Promise<number> {
        const events = new EventView(usage.eventKind(), { store: this.events });

        return Usage.read(account.identifier(payer), meter, period, this.clock.now(), events);
    }

    /** Append an account's uses as the machines measuring them send them, leaving them unrated. */
    async append(payer: string, uses: readonly FinanceUse[]): Promise<void> {
        // send a space's uses as the residency's machine serving it, and a machine's as the machine itself
        const now = this.clock.now();
        const payerId = account.identifier(payer);
        for (const [machineId, sent] of Map.groupBy(uses, (use) => use.machine ?? null)) {
            const sender =
                machineId === null
                    ? await this.#serving()
                    : Caller.of(principal.machine.reference(Scope.universe.id, machineId));
            await this.store.mutate(AccessContext.of(sender, now), {
                id: RequestId.create(this.environment),
                calls: [
                    usage.call("append", {
                        ...usage.in(payerId),
                        items: await Promise.all(sent.map((use) => this.#item(payerId, use))),
                    }),
                ],
            });
        }
    }

    /** Call as the residency's cloud machine serving its accounts' spaces. */
    async #serving(): Promise<Caller> {
        // act as the machine in its residency's servers
        const subject = principal.machine.reference(Scope.universe.id, this.space.machine.id);
        const universe = this.space.universe.storage.database.snapshot();
        const home = await Residency.require(universe, RESIDENCY);

        return { ...Caller.of(subject), groups: [Residency.servers(home.id)] };
    }

    /** Write a use as the usage append method takes it, identified by its name. */
    async #item(payer: Identifier<"account">, use: FinanceUse) {
        return {
            id: await usage.derive(payer, use.name, this.environment.crypto),
            time: EventTime.of(use.time),
            meter: DeclarationReference.key(use.meter),
            sku: use.sku === undefined ? null : DeclarationReference.key(use.sku),
            spaceId: use.space ?? null,
            machineId: use.machine ?? null,
            installationId: null,
            quantity: use.quantity,
            level: use.level ?? null,
            data: { from: use.time, to: use.time },
        };
    }

    /** Resolve each feature an account's sources grant now, with its metered usage. */
    resolutions(scope: string): Promise<Resolution[]> {
        const { home } = this.#implementation;

        return new CustomerEntitlement(this.#store, home, this.#implementation.metering).resolve(
            scope,
        );
    }

    /** Wait until the machine's loop rates the new usage and derives the accounts' entitlements. */
    async idle(): Promise<void> {
        await this.space.simulation.idle();
    }

    /** Rate the new usage in two concurrent takes, as two machines' loops can. */
    async rateTwice(): Promise<void> {
        // build the usage cursor over the store serving now, and take it twice at once
        const usageCursor = present(
            this.#store
                .control()
                .flatMap((set) => set.cursors)
                .find((each) => each.tables.includes(usage.table)),
            "finance's usage cursor",
        );
        await Promise.all([0, 1].map(() => usageCursor.take(reconciliation())));
        await this.space.simulation.idle();
    }

    /** Restart the home's machine, whose loop takes its cursors again at its start. */
    async restart(): Promise<void> {
        await this.space.restart({});
        this.#store = await this.space.store();
        await this.space.simulation.idle();
    }

    /** Write a value as the current version of a home secret, as the operator stepped up. */
    async writeSecret(name: string, value: string): Promise<void> {
        // step the operator up, and find the secret the home's stack declares
        const operator = await this.space.universe.elevate(this.person("dave"));
        const vaults = this.space.person(operator).client(vaultPackage);
        const row = await this.database
            .snapshot()
            .read(secret.table)
            .findFirst({ where: { name } });
        const { id } = present(row, `the home secret ${name}`);

        // store the value as a new version, and make it current
        const version = await vaults.version.create({
            scope: this.home,
            parentId: id,
            requestId: RequestId.create(this.environment),
            value: { encoding: "text", value },
        });
        await vaults.secret.update({
            scope: this.home,
            id,
            requestId: RequestId.create(this.environment),
            currentVersion: version.number,
        });
    }

    /** Send a request to one of finance's mounts in the home, at its path below the package. */
    fetch(request: Request): Promise<Response> {
        return this.space.server(financePackage).fetch(request);
    }

    /** Apply sellers and products through the home's stack, its controllers running. */
    async declare(definition: StackDocument<readonly [typeof financePackage]>): Promise<void> {
        await this.space.serve();
        await this.space.apply([financePackage], definition);
    }

    /** Publish a package release declaring features, meters and SKUs, which becomes its latest. */
    async publish(
        owner: PackageVersion,
        declared: readonly (Feature | Meter | Sku)[],
    ): Promise<void> {
        const build = await TestFinance.release(owner, declared, this.environment.crypto);
        await publishBuild(this.database, this.home, this.platform, build, this.environment);
        this.#published.set(releaseKey(owner.id, owner.version), build);
    }

    /** Build a package release declaring some features, meters and SKUs, as its build does. */
    static async release(
        owner: PackageVersion,
        declared: readonly (Feature | Meter | Sku)[],
        crypto: Crypto,
    ): Promise<BuildReader> {
        const declarations = declared.map((entry) => ({
            name: entry.name,
            ...TestFinance.#describe(entry),
        }));

        return await MemoryBuild.of(owner, declarations, crypto);
    }

    /** Describe a declaration by its kind and the package declaring the kind. */
    static #describe(entry: Feature | Meter | Sku) {
        // describe a feature or a SKU as finance's
        if (entry instanceof Feature) {
            return {
                ...FEATURE_KIND.identity,
                description: describeFeature(entry),
            };
        } else if (entry instanceof Sku) {
            return {
                ...SKU_KIND.identity,
                description: describeSku(entry),
            };
        }

        return {
            ...METER_KIND.identity,
            description: describeMeter(entry),
        };
    }

    /** Stop serving, refusing exceptions no test took. */
    async close(): Promise<void> {
        await this.space[Symbol.asyncDispose]();
        this.telemetry[Symbol.dispose]();
    }
}

/** What finance's server implements in a home, with the home and the metering it runs with. */
interface FinanceDefinition {
    /** The implementation. */
    readonly definition: ReturnType<typeof implementFinance>;
    /** The home finance runs in. */
    readonly home: string;
    /** The metering it rates against. */
    readonly metering: Metering;
}

/** Implement finance in a home, billing through a provider and metering published releases. */
function financeServer(
    published: ReadonlyMap<string, BuildReader>,
    charging: FinanceFixtureOptions["provider"],
    implemented: (definition: FinanceDefinition) => void = () => {},
) {
    return implementPackage(financePackage, (context) => {
        // implement finance, handing the definition to the fixture that steps its cursors
        const provider = charging === undefined ? undefined : charging(context.scope);
        const metered = metering(context, published);
        const definition = implementFinance(context.scope, metered, provider, context);
        implemented({ definition, home: context.scope, metering: metered });

        return definition;
    });
}

/** Meter usage in a home against the published releases. */
function metering(context: ServerContext, published: ReadonlyMap<string, BuildReader>): Metering {
    return new Metering(
        RELEASE_TAG,
        async (packageId, version) =>
            present(
                published.get(releaseKey(packageId, version)),
                `the published build ${version}`,
            ),
        context.environment.telemetry,
    );
}

/** Key a published build by its package and version. */
function releaseKey(packageId: string, version: string): string {
    return `${packageId}@${version}`;
}

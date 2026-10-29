import accountPackage from "@destack/account/package";
import objectPackage from "@destack/object/package";
import { AccessContext, Caller, principal } from "@destack/access";
import { account, organization, Residency } from "@destack/account/object";
import { OPERATOR_EMAIL, type TestPerson } from "@destack/account/test";
import { type Controller, Snapshot } from "@destack/db";
import { reconciliation } from "@destack/db/test";
import type { Crypto, Clock, Environment } from "@destack/environment";
import { type EventStore, EventTime } from "@destack/event";
import { packageObject, tag } from "@destack/forge/object";
import forgePackage from "@destack/forge/package";
import { publishBuild, resolvedRelease } from "@destack/forge/test";
import { notification } from "@destack/notification";
import messagePackage from "@destack/message/package";
import notificationPackage from "@destack/notification/package";
import { TestNotification } from "@destack/notification/test";
import { EventView, PACKAGE_DECLARATION, PACKAGE_EXPORT, type ReaderPage } from "@destack/object";
import { describePackage } from "@destack/object/inspect";
import { implementPackage, type ObjectStore } from "@destack/object/server";
import type { InstalledServerContext } from "@destack/object/workload";
import { serversOf, type TestObjectStore } from "@destack/object/test";
import { describeMeter } from "@destack/observability/inspect";
import type { Meter } from "@destack/observability/meter";
import observabilityPackage from "@destack/observability/package";
import { Package } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import { type BuildReader } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { Digest, Identifier, present, schema } from "@destack/schema";
import type { StackDocument } from "@destack/stack/declare";
import { install } from "@destack/stack/declare";
import { TestPackage, TestSpace } from "@destack/stack/test";
import { defineVault } from "@destack/vault/declare";
import vaultPackage from "@destack/vault/package";
import { TestVault } from "@destack/vault/test";
import { vaultProvider } from "@destack/vault/provider";
import vaultServer, { secretBindable } from "@destack/vault/server";
import { RequestId, Scope, Subject } from "@destack/sync";
import { TestTelemetry } from "@destack/telemetry/test";
import { testHost } from "@destack/test/host";
import { Feature } from "../feature/index.ts";
import { describeFeature, describeSku } from "../inspect/index.ts";
import { type Resolution, usage, Usage, usageLimit } from "../object/index.ts";
import financePackage, { FINANCE_INSTALLATION } from "../package.ts";
import type { PaymentProvider } from "../provider/index.ts";
import type { Period } from "../rate/index.ts";
import { EntitlementController } from "../server/entitlement-controller.ts";
import { Metering } from "../server/metering.ts";
import { implementFinance } from "../server/server.ts";
import { Sku } from "../sku/index.ts";

/** The residency whose home finance serves in. */
const RESIDENCY = "eu";

/** The tag the kit's releases open at, which answers each package's latest. */
const RELEASE_TAG = "latest";

/** The tenancies a plan granting dedicated machines lets an account run. */
const TENANCIES = ["shared", "dedicated", "local"];

/** The origin of the dedicated machines the kit declares. */
const DEDICATED_ORIGIN = "https://dedicated.machine.test/";

/** The people the scenarios act as, by the name of their email; dave operates the platform. */
export const PEOPLE = ["alice", "bob", "carol", "dave", "eve"] as const;

/** A person the scenarios act as. */
export type Person = (typeof PEOPLE)[number];

/** The organization and accounts the scenarios bill, the platform's apart. */
export const ids = {
    acme: organization.identifier("organization-019f5530-8000-7000-8000-000000000301"),
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
    /** The use's identity, unique across spaces. */
    readonly id: string;
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

/** The options of a finance kit. */
export interface FinanceFixtureOptions {
    /** Open the payment provider billing through for the home, none by default. */
    readonly provider?: (home: string) => PaymentProvider;
    /** The moment the test starts at, as the template's world left it by default. */
    readonly start?: number;
}

/** Finance's build, declaring its package, notifications and secrets, as its home installs it. */
const FINANCE: TestPackage = {
    handle: financePackage,
    build: await MemoryBuild.of(
        usageLimit.package,
        [
            {
                kind: PACKAGE_DECLARATION,
                kindPackageId: objectPackage.id,
                name: PACKAGE_EXPORT,
                description: describePackage(financePackage),
            },
            {
                kind: "notification",
                kindPackageId: notification.package.id,
                name: usageLimit.name,
                description: usageLimit.describe(),
            },
            ...TestVault.declarations([], Object.values(financePackage.secrets)),
        ],
        testHost().crypto,
    ),
    server: financeServer(new Map(), undefined),
    connectors: [],
    servers: [vaultServer],
    bindables: [secretBindable],
    providers: [vaultProvider],
};

/** The packages the residency's home installs beside its base ones, as its stack does. */
const PACKAGES: readonly TestPackage[] = [
    ...(await TestNotification.packages(testHost().crypto)),
    FINANCE,
];

/** The platform account's vault in the residency's home, keeping the Stripe secret key. */
const PLATFORM_VAULT = defineVault({ name: "platform", spec: {} });

/** The packages whose types the home's stack declares. */
const HOME_TYPES = [vaultPackage, financePackage] as const;

/** The home's stack, as the residency's declares it: finance bound to the platform's Stripe key. */
const HOME_STACK = {
    vaults: { platform: PLATFORM_VAULT.declare() },
    secrets: { "stripe-secret-key": { parent: "platform", name: "stripe-secret-key" } },
    installations: {
        message: install(messagePackage, {}, { name: "message" }),
        notification: install(notificationPackage, {}, { name: "notification" }),
        finance: install(
            financePackage,
            { "stripe-secret-key": "stripe-secret-key" },
            { name: FINANCE_INSTALLATION },
        ),
    },
};

/** The home the kit's tests copy: its packages, forge's releases and the platform's publisher. */
const HOME_OPTIONS = {
    ...TestPackage.options(PACKAGES),
    publishers: ["platform"],
    packages: [...TestPackage.options(PACKAGES).packages, forgePackage],
    companions: [
        ...TestPackage.options(PACKAGES).companions,
        ...serversOf([packageObject, resolvedRelease, tag]),
    ],
};

/** What the kit's world shares: the platform's account. */
const FinanceWorld = schema.object({
    /** The platform's account in the residency, whose home is the residency's. */
    platform: schema.identifier("account"),
});

/** What the kit's world shares. */
type FinanceWorld = schema.Infer<typeof FinanceWorld>;

/** Finance in the residency's home with Acme's buyer and payer, and Carol's shop. */
export class FinanceFixture {
    /** The residency's home serving finance. */
    readonly space: TestSpace;
    /** The home as its machine serves it, its controllers stepped by the test. */
    readonly served: TestObjectStore;
    /** The payment provider billing through, absent for a kit selling nothing. */
    readonly provider: PaymentProvider | undefined;
    /** The telemetry recording the exceptions finance captures, refused at close unless taken. */
    readonly telemetry: TestTelemetry;
    /** The platform's account in the residency. */
    readonly platform: Identifier<"account">;
    /** The published builds the metering reads, by their manifest's digest. */
    readonly #published: Map<string, BuildReader>;
    /** Finance's controllers, which the test runs where the control loop would. */
    readonly #controllers: readonly Controller[];

    /** Keep a served home with finance and the world the template wrote. */
    private constructor(fields: {
        readonly space: TestSpace;
        readonly served: TestObjectStore;
        readonly provider: PaymentProvider | undefined;
        readonly telemetry: TestTelemetry;
        readonly world: FinanceWorld;
        readonly published: Map<string, BuildReader>;
        readonly controllers: readonly Controller[];
    }) {
        // keep the home, how it is served, and the metering and provider finance runs with
        this.space = fields.space;
        this.served = fields.served;
        this.provider = fields.provider;
        this.telemetry = fields.telemetry;

        // keep the world's accounts and machines, and the published builds
        this.platform = fields.world.platform;
        this.#published = fields.published;
        this.#controllers = fields.controllers;
    }

    /** Serve a copy of the template's home with package releases, billing through a provider. */
    static async open(
        releases: readonly (readonly [Package, readonly (Feature | Meter | Sku)[]])[],
        options: FinanceFixtureOptions = {},
    ): Promise<FinanceFixture> {
        // build each release, which the metering reads by its manifest's digest
        const published = new Map<string, BuildReader>();
        const { crypto } = testHost();
        for (const [owner, declared] of releases) {
            const build = await FinanceFixture.release(owner, declared, crypto);
            published.set(await Digest.json(build.manifest, crypto), build);
        }

        // copy the home with finance billing through the provider and metering against the releases
        const charging = options.provider;
        const implemented = Promise.withResolvers<FinanceDefinition>();
        const server = financeServer(published, charging, (definition) =>
            implemented.resolve(definition),
        );
        const { space: home, world } = await TestSpace.copy(FinanceFixture.template(), {
            ...HOME_OPTIONS,
            servers: HOME_OPTIONS.servers.map((each) =>
                each.package.id === financePackage.id ? server : each,
            ),
        });

        // serve it at the test's start, its controllers stepped by the test, and publish the releases as the platform
        const served = await home.serve({ isControlled: false });
        const { controllers = [] } = await implemented.promise;
        const built = controllers.map((controller) => controller(served.store));
        if (options.start !== undefined) {
            await home.jump(served, options.start);
        }
        for (const build of published.values()) {
            await publishBuild(served.database, home.id, world.platform, build, home.environment);
        }

        return new FinanceFixture({
            space: home,
            served,
            provider: charging === undefined ? undefined : charging(home.id),
            telemetry: new TestTelemetry(home.simulation.telemetry),
            world,
            published,
            controllers: built,
        });
    }

    /** Define the template of the kit's world, which every test copies. */
    static template() {
        return TestSpace.template(HOME_OPTIONS, {
            name: "finance",
            value: FinanceWorld,
            spaces: [{ residency: RESIDENCY }],
            write: (home) => FinanceFixture.#write(home),
        });
    }

    /** Install finance in the residency's home, and create Acme, the shop and their spaces. */
    static async #write(home: TestSpace): Promise<FinanceWorld> {
        // install finance bound to the platform's Stripe key, as the residency's stack does
        const installing = await home.serve();
        await home.apply(installing, HOME_TYPES, HOME_STACK);
        await home.serveInstalled(installing.store, home.id);
        await installing[Symbol.asyncDispose]();

        // sign the people in, alice creating Acme paying with its billing account, and carol her shop
        const { universe } = home;
        const people = new Map<Person, TestPerson>();
        for (const name of PEOPLE) {
            const person =
                name === "dave"
                    ? await universe.operator()
                    : await universe.signIn(FinanceFixture.#email(name));
            people.set(name, person);
        }
        const alice = present(people.get("alice"), "alice");
        const carol = present(people.get("carol"), "carol");
        const bob = present(people.get("bob"), "bob");
        const accounts = alice.client(accountPackage);
        await accounts.organization.create({
            id: ids.acme,
            requestId: RequestId.create(home.environment),
            name: "Acme",
            kind: "business",
            handle: "acme-group",
            residencyId: RESIDENCY,
        });
        for (const [id, handle] of [
            [ids.buyer, "acme"],
            [ids.payer, "acme-billing"],
        ] as const) {
            await accounts.account.create({
                scope: ids.acme,
                id,
                requestId: RequestId.create(home.environment),
                handle,
                name: handle,
                kind: "shared",
                residencyId: RESIDENCY,
            });
        }
        await accounts.organization.update({
            id: ids.acme,
            requestId: RequestId.create(home.environment),
            payer: ids.payer,
        });
        await TestSpace.account(universe, carol, { id: ids.shop, handle: "shop" });

        // let bob view the buyer's account
        await alice.client(objectPackage).relationship.create({
            scope: ids.acme,
            requestId: RequestId.create(home.environment),
            object: account.reference(ids.acme, ids.buyer),
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
        await using served = await home.serve();
        await home.settle(served);
        const platform = await Snapshot.live(served.database)
            .read(account.table)
            .findFirst({ where: { handle: "destack" } });

        return { platform: present(platform, "the platform's account").id };
    }

    /** The residency's home, where finance stores its rows. */
    get home(): Identifier<"space"> {
        return this.space.id;
    }

    /** The served store of the home. */
    get store(): ObjectStore {
        return this.served.store;
    }

    /** The home's database. */
    get database() {
        return this.served.database;
    }

    /** The event log recording the calls and the payers' usage. */
    get events(): EventStore {
        return this.served.store.events;
    }

    /** The world the home's servers run in. */
    get environment(): Environment {
        return this.served.environment;
    }

    /** The clock finance runs at, its world's. */
    get clock(): Clock {
        return this.served.environment.clock;
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

    /** Jump the home's world to a later time, settling what falls due. */
    jump(to: number): Promise<void> {
        return this.space.jump(this.served, to);
    }

    /** Read a person of the scenarios. */
    person(name: Person): TestPerson {
        return this.space.universe.person(FinanceFixture.#email(name));
    }

    /** Read a person's subject. */
    subject(name: Person): Subject {
        return this.person(name).subject;
    }

    /** Address a person by name, dave as the platform's operator. */
    static #email(name: Person): string {
        return name === "dave" ? OPERATOR_EMAIL : `${name}@example.test`;
    }

    /** Append an account's uses, then rate them page by page. */
    async use(payer: string, uses: readonly FinanceUse[]): Promise<void> {
        // append the uses, then rate every page
        await this.append(payer, uses);
        while ((await this.rate(payer)) === 0) {
            continue;
        }
    }

    /** Fold a paying account's usage of a meter as rating reads it, in a period or now. */
    usage(payer: string, meter: Meter, period: Period | undefined): Promise<number> {
        const events = new EventView(usage.eventKind(), this.events);

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
                    ? this.#serving()
                    : Caller.of(principal.machine.reference(Scope.universe.id, machineId));
            await this.store.mutate(AccessContext.of(sender, now), {
                id: RequestId.create(this.environment),
                calls: [
                    usage.call("append", {
                        ...usage.in(payerId),
                        items: sent.map((use) => FinanceFixture.#item(use)),
                    }),
                ],
            });
        }
    }

    /** Append a page of a space's readings as each payer's uses, as its serving machine does. */
    async bill(_owner: string, page: ReaderPage): Promise<void> {
        const sender = this.#serving();
        const now = this.clock.now();
        for (const { accountId, requestId, items } of await Usage.of(
            page,
            this.environment.crypto,
        )) {
            await this.store.mutate(AccessContext.of(sender, now), {
                id: requestId,
                calls: [usage.call("append", { ...usage.in(accountId), items })],
            });
        }
    }

    /** Call as the residency's cloud machine serving its accounts' spaces. */
    #serving(): Caller {
        const subject = principal.machine.reference(Scope.universe.id, this.space.machine.id);

        return { ...Caller.of(subject), groups: [Residency.servers(RESIDENCY)] };
    }

    /** Write a use as the usage append method takes it. */
    static #item(use: FinanceUse) {
        return {
            id: use.id,
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

    /** Find a served controller by its name. */
    #controller(name: string): Controller {
        return present(
            this.#controllers.find((each) => each.name === name),
            `finance's ${name} controller`,
        );
    }

    /** The entitlement derivation, which the test runs where the control loop would. */
    get #entitlements(): EntitlementController {
        const found = this.#controller("entitlement");
        if (!(found instanceof EntitlementController)) {
            throw new TypeError("the entitlement controller is not finance's derivation");
        }

        return found;
    }

    /** Resolve each feature an account's sources grant now, with its metered usage. */
    resolutions(scope: string): Promise<Resolution[]> {
        return this.#entitlements.resolve(scope);
    }

    /** Rate the next page of an account's usage and derive its entitlements, as the loop does. */
    async rate(scope: string): Promise<number | undefined> {
        // rate the page, then derive the entitlements and settle the replicas they change
        const rated = await this.#controller(`${usage.name}/usage`).reconcile(
            scope,
            reconciliation(),
        );
        await this.#entitlements.reconcile(scope);
        await this.space.settle(this.served);

        return rated;
    }

    /** Flush the hot usage of an account into segments, as the event store's controller does. */
    async flush(payer: string): Promise<void> {
        for (const controller of this.events.controllers) {
            await controller.reconcile(`${usage.eventKind().key} ${payer}`, reconciliation());
        }
    }

    /** Rate each payer a restarted usage controller lists until its pages run out. */
    async rateListed(): Promise<readonly string[]> {
        const listed = await this.#controller(`${usage.name}/usage`).list(
            Snapshot.live(this.database),
        );
        for (const payer of listed) {
            while ((await this.rate(payer)) === 0) {
                continue;
            }
        }

        return listed;
    }

    /** Derive every account's entitlements, as the controller does. */
    async entitle(): Promise<void> {
        const entitlements = this.#entitlements;
        for (const key of await entitlements.list()) {
            await entitlements.reconcile(key);
        }
        await this.space.settle(this.served);
    }

    /** Apply sellers and products through the home's stack, its controllers running. */
    async declare(definition: StackDocument<readonly [typeof financePackage]>): Promise<void> {
        await using applying = await this.space.serve();
        await this.space.apply(applying, [financePackage], definition);
    }

    /** Publish a package release declaring features, meters and SKUs, which becomes its latest. */
    async publish(owner: Package, declared: readonly (Feature | Meter | Sku)[]): Promise<void> {
        const build = await FinanceFixture.release(owner, declared, this.environment.crypto);
        await publishBuild(this.database, this.home, this.platform, build, this.environment);
        this.#published.set(await Digest.json(build.manifest, this.environment.crypto), build);
    }

    /** Build a package release declaring some features, meters and SKUs, as its build does. */
    static async release(
        owner: Package,
        declared: readonly (Feature | Meter | Sku)[],
        crypto: Crypto,
    ): Promise<BuildReader> {
        const declarations = declared.map((entry) => ({
            name: entry.name,
            ...FinanceFixture.#describe(entry),
        }));

        return await MemoryBuild.of(owner, declarations, crypto);
    }

    /** Describe a declaration by its kind and the package declaring the kind. */
    static #describe(entry: Feature | Meter | Sku) {
        // describe a feature or a SKU as finance's
        if (entry instanceof Feature) {
            return {
                kindPackageId: financePackage.id,
                kind: "feature",
                description: describeFeature(entry),
            };
        } else if (entry instanceof Sku) {
            return {
                kindPackageId: financePackage.id,
                kind: "sku",
                description: describeSku(entry),
            };
        }

        return {
            kindPackageId: observabilityPackage.id,
            kind: "meter",
            description: describeMeter(entry),
        };
    }

    /** Stop serving, refusing exceptions no test took. */
    async close(): Promise<void> {
        await this.served[Symbol.asyncDispose]();
        await this.space[Symbol.asyncDispose]();
        this.telemetry[Symbol.dispose]();
    }
}

/** What finance's server implements in a home. */
type FinanceDefinition = ReturnType<typeof implementFinance>;

/** Implement finance in a home, billing through a provider and metering published releases. */
function financeServer(
    published: ReadonlyMap<string, BuildReader>,
    charging: FinanceFixtureOptions["provider"],
    implemented: (definition: FinanceDefinition) => void = () => {},
) {
    return implementPackage(financePackage, (context) => {
        // implement finance, handing the definition to the kit that steps its controllers
        const provider = charging === undefined ? undefined : charging(context.scope);
        const definition = implementFinance(context.scope, metering(context, published), provider);
        implemented(definition);

        return definition;
    });
}

/** Meter usage in a home against the published releases. */
function metering(
    context: InstalledServerContext,
    published: ReadonlyMap<string, BuildReader>,
): Metering {
    return new Metering(
        RELEASE_TAG,
        async (_packageId, manifest) =>
            present(published.get(manifest), `the published build ${manifest}`),
        context.environment.telemetry,
    );
}

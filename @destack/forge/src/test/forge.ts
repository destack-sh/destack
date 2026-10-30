import { install } from "@destack/stack/declare";
import { Account, account, type ResidencyCode } from "@destack/account/object";
import { principal } from "@destack/access";
import { integration, Issuer } from "@destack/connection/object";
import connectionPackage, { CONNECTION_INSTALLATION } from "@destack/connection/package";
import connectionServer, { connectionShape } from "@destack/connection/server";
import accountPackage from "@destack/account/package";
import type { TestPerson } from "@destack/account/test";
import { PackageClient, PackageStore } from "@destack/build/store";
import { Forge } from "../server/forge.ts";
import { Snapshot, type Select } from "@destack/db";
import type { Environment } from "@destack/environment";
import type { Simulation } from "@destack/environment/simulation";
import {
    ClaimLookup,
    claimTable,
    InstallationMount,
    OBJECT_DECLARATION,
    ObjectDescription,
    PACKAGE_DECLARATION,
    PACKAGE_EXPORT,
    PackageMount,
} from "@destack/object";
import { createClient } from "@destack/object/client";
import { describePackage } from "@destack/object/inspect";
import objectPackage from "@destack/object/package";
import type { TestObjectStore } from "@destack/object/test";
import { Package, PackageName, type PackageId } from "@destack/package";
import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { type Digest, Identifier, present, schema } from "@destack/schema";
import { join } from "node:path";
import settingPackage from "@destack/setting/package";
import { describeSetting } from "@destack/setting/inspect";
import { binding } from "@destack/stack/object";
import { TestPackage, TestSpace } from "@destack/stack/test";
import { RequestId, type Subject } from "@destack/sync";
import { defineVault } from "@destack/vault/declare";
import { RESOURCE_DECLARATION, RESOURCE_KIND_PACKAGE_ID } from "@destack/resource";
import { secret, vault } from "@destack/vault/object";
import vaultPackage from "@destack/vault/package";
import { TestVault } from "@destack/vault/test";
import vaultServer from "@destack/vault/server";
import { onTestFinished } from "@destack/test";
import { testHost } from "@destack/test/host";
import { LocalGitStorage, localGitProvider } from "../local/index.ts";
import { issuer, packageObject, type Release } from "../object/index.ts";
import forgePackage, { FORGE_INSTALLATION } from "../package.ts";
import forgeServer from "../server.ts";
import { claimFollow, standingFollow } from "../server/index.ts";
import { credentialVault, github, repositoryStorage } from "../stack/index.ts";
import { gitStorage } from "../storage/index.ts";
import { GITHUB_API } from "../github/index.ts";
import { APP_ID, GITHUB_HOST, GitHubStandIn, githubPrivateKey, WEBHOOK_SECRET } from "./github.ts";

/** The node GitHub's stand-in answers on in the world's network. */
const GITHUB_NODE = "github";

/** The port GitHub's hosts answer HTTPS at. */
const HTTPS_PORT = 443;

/** The host name the fixture's forge listens at for package managers on the host. */
const LOOPBACK = "127.0.0.1";

/** The origin the fixture's requests name, which the served home answers at any host. */
const ORIGIN = "http://forge.test";

/** The residency whose home the fixture's forge serves in. */
export const RESIDENCY = "eu";

/** The residencies of the fixture's universe: the forge's, and another the forge serves none of. */
const RESIDENCIES = [
    { code: RESIDENCY, name: "Europe" },
    { code: "us", name: "United States" },
];

/** The account the fixture's owner publishes packages under, which scopes their npm names. */
export const ACCOUNT_ID = schema
    .identifier("account")
    .parse("account-00000000-0000-7000-8000-000000000001");

/** The handle of the publishing account, the scope of its packages' npm names. */
const ACCOUNT_HANDLE = "example";

/** The email the publishing account's owner signs in with. */
const OWNER_EMAIL = "owner@example.test";

/** The email a person without access to the account signs in with. */
const STRANGER_EMAIL = "stranger@example.test";

/** The tenants the templates sign in and create once, as most tests take one or two. */
const TEMPLATE_TENANTS = 2;

/** The tunnel the machines the fixture enrolls publish through the residency's relay. */
const MACHINE_ORIGIN = "https://machine.relay.test/";

/** The vault the residency's stack declares for the platform's secrets. */
const PLATFORM_VAULT = defineVault({ name: "platform", spec: {} });

/** The forge as the machine ships it, beside the vault, with Git storage below its files. */
export const FORGE: TestPackage = {
    handle: forgePackage,
    build: await MemoryBuild.of(
        Package.of(forgePackage),
        [
            {
                kind: PACKAGE_DECLARATION,
                kindPackageId: objectPackage.id,
                name: PACKAGE_EXPORT,
                description: describePackage(forgePackage),
            },
            ...Object.values(forgePackage.objects).map((type) => ({
                kind: OBJECT_DECLARATION,
                kindPackageId: objectPackage.id,
                name: type.name,
                description: ObjectDescription.of(type),
            })),
            {
                kind: "setting",
                kindPackageId: settingPackage.id,
                name: github.name,
                description: describeSetting(github),
            },
            ...Object.values(forgePackage.resources).map((resource) => ({
                kind: RESOURCE_DECLARATION,
                kindPackageId: RESOURCE_KIND_PACKAGE_ID,
                name: resource.name,
                description: {
                    name: resource.name,
                    kind: resource.kind,
                    spec: resource.spec,
                    ...resource.state(),
                },
            })),
            ...TestVault.declarations([], Object.values(forgePackage.secrets)),
        ],
        testHost().crypto,
    ),
    server: forgeServer,
    connectors: [],
    servers: [vaultServer],
    providers: [],
};

/** Connections as the machine ships them, beside the vault keeping their credentials. */
export const CONNECTION: TestPackage = {
    handle: connectionPackage,
    build: await MemoryBuild.of(
        Package.of(connectionPackage),
        [
            {
                kind: PACKAGE_DECLARATION,
                kindPackageId: objectPackage.id,
                name: PACKAGE_EXPORT,
                description: describePackage(connectionPackage),
            },
            ...Object.values(connectionPackage.objects).map((type) => ({
                kind: OBJECT_DECLARATION,
                kindPackageId: objectPackage.id,
                name: type.name,
                description: ObjectDescription.of(type),
            })),
        ],
        testHost().crypto,
    ),
    server: connectionServer,
    connectors: [],
    servers: [vaultServer],
    providers: [],
};

/** The vault of an account's home that its connections keep their credentials in. */
const CONNECTION_VAULT = defineVault({ name: "credentials", spec: {} });

/** An account's home stack as the platform declares it: its connections and their vault. */
const ACCOUNT_STACK = {
    vaults: { credentials: { ...CONNECTION_VAULT.declare(), scope: "enclosing" as const } },
    installations: {
        connection: install(connectionPackage, {}, { name: CONNECTION_INSTALLATION }),
    },
};

/** Where the fixture GitHub App's setup returns to after an installation. */
const SETUP_URL = "https://forge.test/github/setup";

/** The packages whose types the home's stack declares. */
const HOME_TYPES = [vaultPackage, forgePackage, gitStorage] as const;

/** The home's stack, as the residency's declares it: the forge with its resources and issuer. */
const HOME_STACK = {
    vaults: { platform: PLATFORM_VAULT.declare(), credentials: credentialVault.declare() },
    gitStorages: { repositories: repositoryStorage.declare() },
    issuers: { github: {} },
    secrets: {
        "github-webhook": { parent: "platform", name: "github-webhook" },
        "github-private-key": { parent: "platform", name: "github-private-key" },
    },
    installations: {
        forge: install(
            forgePackage,
            {
                repositories: "repositories",
                credentials: "credentials",
                "github-webhook": "github-webhook",
                "github-private-key": "github-private-key",
            },
            { name: FORGE_INSTALLATION },
        ),
    },
};

/** The residency's home its tests copy: the forge with the vault's tables and Git storage. */
const HOME_OPTIONS = {
    residencies: RESIDENCIES,
    ...TestPackage.options([FORGE]),
    packages: [...TestPackage.options([FORGE]).packages, vaultPackage],
    tables: gitStorage.tables,
    // replicate the connections the forge follows as its installation, beside its names and callers
    follows: [{ shape: connectionShape, keys: [] }, claimFollow, standingFollow],
    policies: Object.values(forgePackage.objects).map((object) => object.policy),
    resources: (directory: string, environment: Environment<"files" | "process" | "host">) => ({
        providers: [localGitProvider(directory, gitStorage, environment)],
        connectors: [LocalGitStorage.connector(directory, environment)],
    }),
};

/** What the fixture's world shares: the forge installation in the residency's home. */
const ForgeWorld = schema.object({
    /** The forge's installation. */
    installation: schema.identifier("installation"),
    /** The accounts of the tenants the template created, in order. */
    tenants: schema.array(schema.identifier("account")),
});

/** What the fixture's world shares. */
type ForgeWorld = schema.Infer<typeof ForgeWorld>;

/** The template of the residency's home serving the forge and its GitHub App. */
export const FORGE_TEMPLATE = TestSpace.template(HOME_OPTIONS, {
    name: "forge",
    value: ForgeWorld,
    spaces: [{ residency: RESIDENCY }],
    write: (home) => TestForge.write(home),
});

/** An account of one test with its owner, a viewer of its repositories and a stranger to it. */
export interface Tenant {
    /** The account. */
    readonly accountId: Identifier<"account">;
    /** The account's owner. */
    readonly owner: TestPerson;
    /** A person viewing the account, who reads and pulls its repositories. */
    readonly reader: TestPerson;
    /** A person without access to the account. */
    readonly stranger: TestPerson;
}

/** A package the forge published: its build and release. */
export interface Published {
    /** The build the release was published from. */
    readonly reader: BuildReader;
    /** The package's identity. */
    readonly packageId: PackageId;
    /** The digest of the release's build manifest. */
    readonly manifest: Digest;
    /** The release as its creation returned it. */
    readonly release: Release;
}

/** The forge installed in a residency's home, publishing as the owner of an account. */
export class TestForge {
    /** The residency's home serving the forge. */
    readonly home: TestSpace;
    /** The home as its machine serves it. */
    readonly served: TestObjectStore;
    /** The forge's installation in the home. */
    readonly installation: Identifier<"installation">;
    /** GitHub, as the world's stand-in answers at its hosts. */
    readonly github: GitHubStandIn;
    /** The owner of the publishing account, on the client they last signed in on. */
    #owner: TestPerson;
    /** The accounts of the template's tenants, taken in order. */
    readonly #templated: readonly Identifier<"account">[];
    /** The tenants the fixture took or created, which number their people and handles. */
    #tenants = 0;
    /** The packages the fixture created, by identity. */
    readonly #created = new Set<string>();

    /** Keep a served home and the forge it installs. */
    private constructor(fields: {
        readonly home: TestSpace;
        readonly served: TestObjectStore;
        readonly installation: Identifier<"installation">;
        readonly github: GitHubStandIn;
        readonly owner: TestPerson;
        readonly tenants: readonly Identifier<"account">[];
    }) {
        // keep the home, how it is served, the forge's installation, GitHub, the publisher and the tenants
        this.home = fields.home;
        this.served = fields.served;
        this.installation = fields.installation;
        this.github = fields.github;
        this.#owner = fields.owner;
        this.#templated = fields.tenants;
    }

    /** The owner of the publishing account. */
    get owner(): TestPerson {
        return this.#owner;
    }

    /** Sign the owner in again on a new client, as a person does once their session lapsed. */
    async signOwnerIn(): Promise<TestPerson> {
        this.#owner = await this.signIn(OWNER_EMAIL);

        return this.#owner;
    }

    /** The residency's home, which stores every package of its residency. */
    get space(): Identifier<"space"> {
        return this.home.id;
    }

    /** The npm registry's URL the forge names in its documents, below its installation's mount. */
    get npm(): string {
        const mounted = InstallationMount.url(
            this.home.host.machine.origin,
            this.space,
            this.installation,
        );

        return `${mounted}${PackageMount.path(forgePackage.id)}${forgePackage.adapters.npm.path}`;
    }

    /** Write the npm configuration selecting the forge's registry for a scope as the owner. */
    async npmrc(scope: string): Promise<string> {
        // select the registry for the scope, authenticated as the owner calling the forge in its home
        const url = new URL(`${this.npm}/`);
        const { accessToken } = await this.owner.exchange(forgePackage.id, {
            partition: this.space,
        });

        return `${scope}:registry=${url.href}\n//${url.host}${url.pathname}:_authToken=${accessToken}\n`;
    }

    /** The origin the fixture's requests name. */
    get origin(): string {
        return ORIGIN;
    }

    /** The world the home's servers run in. */
    get environment(): Environment {
        return this.served.environment;
    }

    /** Serve a copy of the template's home with the forge installed, in a fork of its world. */
    static async open(): Promise<TestForge> {
        // copy the home, with GitHub answering in its world
        const { space: home, world } = await TestSpace.copy(FORGE_TEMPLATE, HOME_OPTIONS);
        const standIn = TestForge.#github(home);

        // serve it with the forge
        const served = await home.serve();
        const owner = home.universe.person(OWNER_EMAIL);
        const { installation, tenants } = world;

        return new TestForge({ home, served, installation, github: standIn, owner, tenants });
    }

    /** Answer as GitHub at its API and Git hosts on the world's network until the test ends. */
    static #github(home: TestSpace): GitHubStandIn {
        const standIn = new GitHubStandIn(home.environment);
        for (const hostname of [GITHUB_API.hostname, GITHUB_HOST]) {
            const listener = home.simulation.network.server(GITHUB_NODE).http({
                hostname,
                port: HTTPS_PORT,
                fetch: (request) => standIn.fetch(request),
            });
            onTestFinished(() => listener.stop(true));
        }

        return standIn;
    }

    /** Run the forge's GitHub App: its key and webhook secret, and its setting. */
    static async #runGitHubApp(home: TestSpace): Promise<void> {
        // write the App's key and webhook secret into the platform vault as the operator stepped up
        const configuring = await home.serve();
        const operator = await home.universe.elevate(await home.universe.operator());
        const vaults = home.person(configuring, operator).client(vaultPackage);
        const values = {
            "github-private-key": githubPrivateKey(),
            "github-webhook": WEBHOOK_SECRET,
        };
        for (const [name, value] of Object.entries(values)) {
            const row = await Snapshot.live(configuring.database)
                .read(secret.table)
                .findFirst({ where: { name } });
            const { id } = present(row, `the platform secret ${name}`);
            const version = await vaults.version.create({
                scope: home.id,
                parentId: id,
                requestId: RequestId.create(home.environment),
                value: { encoding: "text", value },
            });
            await vaults.secret.update({
                scope: home.id,
                id,
                requestId: RequestId.create(home.environment),
                currentVersion: version.number,
            });
        }

        // select the App in the forge's setting, then stop serving for the forge to restart with it
        await home.apply(configuring, HOME_TYPES, {
            ...HOME_STACK,
            settings: {
                github: {
                    ...github.reference,
                    value: { kind: "app", appId: APP_ID },
                    mode: "set",
                },
            },
        });
        await configuring[Symbol.asyncDispose]();
    }

    /** Serve a copy of the forge on a loopback port of the host, for package managers. */
    static async listen(): Promise<TestForge> {
        // listen first, for the machine to publish the port as where it answers
        const opened = Promise.withResolvers<TestForge>();
        const listener = testHost().server.http({
            hostname: LOOPBACK,
            port: 0,
            fetch: async (request) => (await opened.promise).home.gateway.answer(request),
        });
        onTestFinished(() => listener.stop(true));

        // copy the home answering there, and serve it with the forge
        const origin = listener.url.origin;
        const { space: home, world } = await TestSpace.copy(FORGE_TEMPLATE, {
            ...HOME_OPTIONS,
            origin,
        });
        const standIn = TestForge.#github(home);
        const served = await home.serve();
        const owner = home.universe.person(OWNER_EMAIL);
        const { installation, tenants } = world;
        const forge = new TestForge({
            home,
            served,
            installation,
            github: standIn,
            owner,
            tenants,
        });
        opened.resolve(forge);

        return forge;
    }

    /** Write the world: the forge, its GitHub App, the publishing account and the tenants. */
    static async write(home: TestSpace): Promise<ForgeWorld> {
        // install the forge with its vaults, bucket, Git storage and secrets, as the residency's stack declares it
        const { issuers: _issuers, ...declared } = HOME_STACK;
        const born = await home.serve();
        await home.apply(born, HOME_TYPES, declared);
        await home.serveInstalled(born.store, home.id);
        await born[Symbol.asyncDispose]();

        // declare the GitHub App's issuer once the home serves the forge's types, as the machine's restart does
        const installing = await home.serve();
        await home.apply(installing, HOME_TYPES, HOME_STACK);
        await installing.simulation.idle();

        // create the publishing account as its owner, then stop serving
        const owner = await home.universe.signIn(OWNER_EMAIL);
        await owner.client(accountPackage).account.create({
            scope: owner.subject.id,
            id: ACCOUNT_ID,
            requestId: RequestId.create(home.environment),
            handle: ACCOUNT_HANDLE,
            name: "Example",
            kind: "shared",
            residencyId: RESIDENCY,
        });
        // sign in and create the tenants most tests take, and the stranger to them
        const tenants: Identifier<"account">[] = [];
        for (let index = 1; index <= TEMPLATE_TENANTS; index += 1) {
            const created = await TestForge.#createTenant(home, `tenant-${index}`, RESIDENCY);
            tenants.push(created.accountId);
        }
        await home.universe.signIn(STRANGER_EMAIL);
        await home.simulation.idle();
        const installation = await TestForge.#installation(installing);
        await installing[Symbol.asyncDispose]();

        // run the GitHub App, as the platform does
        await TestForge.#runGitHubApp(home);

        return { installation, tenants };
    }

    /** Read the forge's installation in the home from its binding of the repository storage. */
    static async #installation(served: TestObjectStore): Promise<Identifier<"installation">> {
        const rows = await Snapshot.live(served.database)
            .read(binding.table)
            .findMany({ where: { packageId: forgePackage.id, name: repositoryStorage.name } });

        return present(rows[0], "the forge's repository storage binding").installationId;
    }

    /** Call the forge as a person with their client's token, or anonymously. */
    client(person?: TestPerson) {
        return createClient(forgePackage, this.endpoint(person), this.environment);
    }

    /** Address the forge's package as a person with their token, or anonymously. */
    endpoint(person?: TestPerson): {
        readonly url: string;
        readonly fetch: (request: Request) => Promise<Response>;
    } {
        // sign a person's requests with their token, and send anonymous ones as they are
        const server = this.served.server(forgePackage);
        const signed =
            person === undefined
                ? (request: Request) => server.fetch(request)
                : this.home.person(this.served, person).endpoint(forgePackage).fetch;

        return { url: ORIGIN, fetch: signed };
    }

    /** Read one of the forge's adapters below its path as a person, or anonymously. */
    fetch(path: string, init: RequestInit = {}, person?: TestPerson): Promise<Response> {
        const { url, fetch } = this.endpoint(person);

        return fetch(new Request(`${url}${path}`, init));
    }

    /** Publish a build as the owner, its units and referrers from a store, creating its package. */
    async publish(
        reader: BuildReader,
        visibility: Select<typeof packageObject.table>["visibility"] = "private",
        store?: PackageStore,
    ): Promise<Published> {
        // create the package under the build's identity and name once
        const publisher = this.client(this.owner);
        const packageId = PackageManifest.package(reader.manifest).id;
        const { local: name } = PackageName.split(reader.manifest.package.name);
        if (!this.#created.has(packageId)) {
            await publisher.package.create({
                spaceId: this.space,
                accountId: ACCOUNT_ID,
                id: packageId,
                requestId: RequestId.create(this.environment),
                name,
                visibility,
            });
            this.#created.add(packageId);
        }

        // push the build, after its npm units from a store, whose runtime builds the forge derives itself
        const uploads = this.builds(packageId);
        const manifest = await PackageStore.digest(reader.manifest, this.environment.crypto);
        if (store === undefined) {
            await uploads.push(reader);
        } else {
            for (const subject of [...(await store.units(manifest, packageId)), manifest]) {
                await uploads.push(subject === manifest ? reader : await store.contents(subject));
            }
        }
        const release = await publisher.release.create({
            spaceId: this.space,
            requestId: RequestId.create(this.environment),
            parentId: packageId,
            manifest,
        });

        return { reader, packageId, manifest: release.manifest, release };
    }

    /** Push a build as a person, the owner by default, returning its manifest's digest. */
    push(reader: BuildReader, person: TestPerson = this.owner): Promise<Digest> {
        return this.builds(PackageManifest.package(reader.manifest).id, person).push(reader);
    }

    /** Connect to the forge's uploads below a package as a person, the owner by default. */
    builds(packageId: string, person: TestPerson = this.owner): PackageClient {
        const { url, fetch } = this.endpoint(person);

        return new PackageClient(
            `${url}${forgePackage.adapters.builds.path}/${packageId}/`,
            fetch,
            this.environment.crypto,
        );
    }

    /** Open the store of builds and npm archives in the forge home's blobs. */
    async store(): Promise<PackageStore> {
        return Forge.openStore({ blobs: this.served.store.blobs, environment: this.environment });
    }

    /** Sign a person in at the universe, such as a stranger to the account. */
    signIn(email: string): Promise<TestPerson> {
        return this.home.universe.signIn(email);
    }

    /** Restore the person without access to the account, whom the template signed in. */
    stranger(): TestPerson {
        return this.home.universe.person(STRANGER_EMAIL);
    }

    /** Locate the forge's releases as a machine's registry reads them, as a person or the owner. */
    registry(person: TestPerson = this.owner): {
        readonly url: string;
        readonly space: string;
        readonly fetch: (request: Request) => Promise<Response>;
    } {
        return { url: ORIGIN, space: this.space, fetch: this.endpoint(person).fetch };
    }

    /** Take the template's next tenant of its residency, else create an account of a new owner. */
    async tenant(residency: ResidencyCode = RESIDENCY): Promise<Tenant> {
        // take the template's next tenant, restoring its owner and viewer
        this.#tenants += 1;
        const handle = `tenant-${this.#tenants}`;
        const templated = residency === RESIDENCY ? this.#templated[this.#tenants - 1] : undefined;
        const { universe } = this.home;
        if (templated !== undefined) {
            return {
                accountId: templated,
                owner: universe.person(`${handle}@example.test`),
                reader: universe.person(`${handle}-reader@example.test`),
                stranger: this.stranger(),
            };
        }

        // create one past the template's, once the home has it
        const created = await TestForge.#createTenant(
            this.home,
            `tenant-${this.#tenants + TEMPLATE_TENANTS}`,
            residency,
        );
        await this.simulation.idle();

        return { ...created, stranger: this.stranger() };
    }

    /** Sign an account's owner and viewer in, and create the account as its owner. */
    static async #createTenant(
        home: TestSpace,
        handle: string,
        residency: ResidencyCode,
    ): Promise<Omit<Tenant, "stranger">> {
        // sign the owner and the viewer in, and create the account as its owner
        const owner = await home.universe.signIn(`${handle}@example.test`);
        const reader = await home.universe.signIn(`${handle}-reader@example.test`);
        const created = await owner.client(accountPackage).account.create({
            scope: owner.subject.id,
            requestId: RequestId.create(home.environment),
            handle,
            name: "Tenant",
            kind: "shared",
            residencyId: residency,
        });

        // let the viewer read and pull the account's repositories
        await owner.client(objectPackage).relationship.create({
            scope: owner.subject.id,
            requestId: RequestId.create(home.environment),
            object: account.reference(owner.subject.id, created.id),
            relation: "viewer",
            subject: reader.subject,
        });

        return { accountId: created.id, owner, reader };
    }

    /** Connect a tenant's account to a GitHub App installation as its owner, bound to the forge. */
    async connect(tenant: Tenant, installation: number): Promise<Identifier<"connection">> {
        // host the account's home with its connections and their vault in the account
        const home = await TestSpace.open({
            world: this.home,
            machine: this.home.machine,
            owner: tenant.owner,
            account: tenant.accountId,
            id: Account.home(tenant.accountId),
            ...TestPackage.options([CONNECTION]),
        });
        const born = await home.serve();
        await home.apply(born, [vaultPackage], ACCOUNT_STACK);
        await home.serveInstalled(born.store, home.id);
        await born[Symbol.asyncDispose]();

        // serve it again with the connections installed, as the machine's restart does
        const served = await home.serve();
        const row = await Snapshot.live(served.database)
            .read(vault.table)
            .findFirst({ where: { scope: tenant.accountId } });
        const credentials = vault.reference(
            tenant.accountId,
            present(row, "the account's vault").id,
        );

        // register the forge's App and create the installation's connection as the owner
        const scope = tenant.accountId;
        const owner = home.person(served, tenant.owner, tenant.accountId).client(connectionPackage);
        const id = await Issuer.declared({ id: this.space }, "github", this.environment.crypto);
        const app = await owner.integration.create({
            scope,
            requestId: RequestId.create(this.environment),
            issuer: {
                kind: "installation",
                package: Package.of(forgePackage),
                name: FORGE_INSTALLATION,
                issuer: issuer.reference(this.space, id),
            },
            endpoints: {
                installation: `https://${GITHUB_HOST}/apps/fixture/installations/new`,
                isIssuerReturned: false,
            },
            clientId: APP_ID,
            authentication: "none",
            redirectUris: [SETUP_URL],
        });
        const created = await owner.connection.create({
            scope,
            requestId: RequestId.create(this.environment),
            protocol: { kind: "installation" },
            integration: integration.reference(scope, app.id),
            vault: credentials,
        });

        // authorize it, complete GitHub's setup redirect, and bind it to the forge
        const { url } = await owner.connection.authorize({
            scope,
            id: created.id,
            requestId: RequestId.create(this.environment),
            redirectUri: SETUP_URL,
        });
        const state = present(new URL(url).searchParams.get("state"), "the installation's state");
        await owner.connection.complete({
            scope,
            id: created.id,
            requestId: RequestId.create(this.environment),
            parameters: { installation_id: String(installation), setup_action: "install", state },
        });
        await owner.connection.bindConsumer({
            scope,
            id: created.id,
            requestId: RequestId.create(this.environment),
            consumer: principal.installation.reference(this.space, this.installation),
        });
        await this.simulation.idle();

        return created.id;
    }

    /** Enroll a machine of an account as a person, published through the residency's relay. */
    async machine(
        accountId: Identifier<"account">,
        person: TestPerson,
    ): Promise<Identifier<"machine">> {
        const enrolled = await this.home.universe.enroll(person, accountId, MACHINE_ORIGIN);
        await this.simulation.idle();

        return enrolled.id;
    }

    /** Create a space of an account as a person, once the home replicates it. */
    async createSpace(
        accountId: Identifier<"account">,
        person: TestPerson,
    ): Promise<Identifier<"space">> {
        // create the space as the person, then run the world idle to place it
        const id = Identifier.create("space", this.environment);
        await person.client(accountPackage).space.create({
            accountId,
            id,
            requestId: RequestId.create(this.environment),
            name: `space-${id.slice(-12)}`,
        });
        await this.simulation.idle();

        return id;
    }

    /** Call the forge as a principal its stores verify, such as a machine or a space. */
    as(subject: Subject) {
        return this.served.as(subject).client(forgePackage);
    }

    /** Open the forge's platform repositories as its bound Git storage keeps them. */
    async storage(): Promise<LocalGitStorage> {
        const rows = await Snapshot.live(this.served.database)
            .read(binding.table)
            .findMany({
                where: { installationId: this.installation, name: repositoryStorage.name },
            });
        const target = present(rows[0], "the forge's Git storage binding").target;

        return new LocalGitStorage(join(this.home.resources, target), this.home.environment);
    }

    /** Look up the universe's names, such as the claims of packages' identities. */
    get claims(): ClaimLookup {
        return ClaimLookup.of(Snapshot.live(this.home.universe.storage.database).read(claimTable));
    }

    /** The simulation running the forge's world. */
    get simulation(): Simulation {
        return this.home.simulation;
    }

    /** Read the build of a package's release, by version or tag, as an anonymous reader. */
    async read(packageId: PackageId, release: string): Promise<BuildReader> {
        const resolved = await this.client().release.resolve({
            spaceId: this.space,
            parentId: packageId,
            release,
        });

        return (await this.store()).contents(resolved.manifest);
    }
}

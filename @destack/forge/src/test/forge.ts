import {
    BuildLayer,
    type BuildReader,
    BuildImage,
    PackageManifest,
} from "@destack/package/manifest";
import { Account, account, type RegionCode } from "@destack/account/object";
import accountPackage from "@destack/account/package";
import type { TestPerson } from "@destack/account/test";
import { RemoteBuildStore, BuildStore } from "@destack/build/store";
import { TestBuildStore } from "@destack/build/test";
import { Forge } from "../server/forge.ts";
import type { Select } from "@destack/db";
import type { Environment } from "@destack/environment";
import type { Simulation } from "@destack/environment/simulation";
import { ClaimLookup, claimTable, type MethodInput } from "@destack/object";
import { createClient } from "@destack/object/client";
import registryPackage from "@destack/registry/package";
import objectPackage from "@destack/object/package";
import { PackageName, type PackageId, type PackageRelease } from "@destack/package";
import { type Digest, Identifier, present, schema } from "@destack/schema";
import { join } from "node:path";
import { installationRevision } from "@destack/stack/object";
import { FORGE_HOME_OPTIONS, OWNER_EMAIL, TestSpace, ids } from "@destack/machine/test";
import { RequestId, type Subject } from "@destack/sync";
import { LocalGitStorage } from "@destack/git/local";
import { TEST_COMMIT } from "./release.ts";
import { LATEST_TAG, packageObject, type Release } from "@destack/registry/object";
import forgePackage from "../package.ts";
import { repositoryStorage } from "../stack/index.ts";

/** The origin of the test requests. */
const ORIGIN = "http://forge.test";

/** The region of the fixture's accounts. */
export const REGION = "eu";

/** The regions of the fixture's universe: its accounts' region, and another one. */
const REGIONS = [
    {
        code: REGION,
        name: "Europe",
        relay: `https://relay-${REGION}.machine.test`,
        countries: ["DE"],
    },
    {
        code: "us",
        name: "United States",
        relay: "https://relay-us.machine.test",
        countries: ["US"],
    },
];

/** The account the fixture's owner publishes packages under, the machine fixtures' own. */
export const ACCOUNT_ID = ids.account;

/** The handle of the publishing account, the scope of its packages' npm names. */
const ACCOUNT_HANDLE = "example";

/** The email a person without access to the account signs in with. */
const STRANGER_EMAIL = "stranger@example.test";

/** The tenants the templates sign in and create once, as most tests take one or two. */
const TEMPLATE_TENANTS = 2;

/** The tunnel the machines the fixture enrolls publish through the region's relay. */
const MACHINE_ORIGIN = "https://machine.relay.test/";

/** The options of a home with vault, connection and forge, in a universe of two regions. */
export const FORGE_WORLD_OPTIONS = {
    ...FORGE_HOME_OPTIONS,
    regions: REGIONS,
    // test the forge's own releases alone, without the machine kit's fixture package
    isPublishingFixtures: false,
};

/** What the fixture's world shares: the forge installation in the publisher's home. */
const ForgeWorld = schema.object({
    /** The forge's installation. */
    installation: schema.identifier("installation"),
    /** The accounts of the tenants the template created, in order. */
    tenants: schema.array(schema.identifier("account")),
});

/** What the fixture's world shares. */
type ForgeWorld = schema.Infer<typeof ForgeWorld>;

/** The publishing account's home, whose forge serves its packages. */
export const PUBLISHER_HOME = { id: Account.home(ACCOUNT_ID), account: ACCOUNT_ID, name: "home" };

/** The template of the publishing account's home serving the forge. */
export const FORGE_TEMPLATE = TestSpace.template(FORGE_WORLD_OPTIONS, {
    name: "forge",
    value: ForgeWorld,
    spaces: [PUBLISHER_HOME],
    write: (home) => TestForge.write(home),
});

/** The account of the space following the forge, apart from the account publishing at it. */
export const FOLLOWER_ACCOUNT_ID = schema
    .identifier("account")
    .parse("account-00000000-0000-7000-8000-000000000002");

/** The home of the account following its own forge, beside the publishing account's. */
export const FOLLOWER_HOME = {
    id: Account.home(FOLLOWER_ACCOUNT_ID),
    account: FOLLOWER_ACCOUNT_ID,
    name: "home",
};

/** The publishing account's home beside the follower's home and its following space. */
export const FOLLOW_TEMPLATE = TestSpace.template(FORGE_WORLD_OPTIONS, {
    name: "forge-follow",
    value: ForgeWorld,
    spaces: [
        PUBLISHER_HOME,
        FOLLOWER_HOME,
        // the follower's space serves none of the homes' packages, following its home forge's rows
        { id: ids.space, account: FOLLOWER_ACCOUNT_ID, name: "space", packages: [] },
    ],
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
    /** The forge in the account's own home, which keeps its repositories and packages. */
    readonly forge: TestForge;
}

/** A package the forge published: its build and release. */
export interface Published {
    /** The build the release was published from. */
    readonly reader: BuildReader;
    /** The package's identity. */
    readonly packageId: PackageId;
    /** The digest of the release's build manifest. */
    readonly manifest: Digest;
    /** The digest of the image the owner pushed, which the release records. */
    readonly image: Digest;
    /** The release as the forge resolves it by version once published. */
    readonly release: Release;
}

/** The forge installed in an account's home, publishing as the owner of the account. */
export class TestForge {
    /** The account's home serving the forge. */
    readonly home: TestSpace;
    /** The forge's installation in the home. */
    readonly installation: Identifier<"installation">;
    /** The owner of the publishing account, on the client they last signed in on. */
    #owner: TestPerson;
    /** The accounts of the template's tenants, taken in order. */
    readonly #templated: readonly Identifier<"account">[];
    /** The tenants the fixture took or created, which number their people and handles. */
    #tenants = 0;

    /** Keep the home and the forge it installs. */
    private constructor(fields: {
        readonly home: TestSpace;
        readonly installation: Identifier<"installation">;
        readonly owner: TestPerson;
        readonly tenants: readonly Identifier<"account">[];
    }) {
        // keep the home, the forge's installation, the publisher and the tenants
        this.home = fields.home;
        this.installation = fields.installation;
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

    /** The account's home, which stores the account's packages. */
    get space(): Identifier<"space"> {
        return this.home.id;
    }

    /** The origin the fixture's requests name. */
    get origin(): string {
        return ORIGIN;
    }

    /** The world the home's servers run in. */
    get environment(): Environment {
        return this.home.environment;
    }

    /** Serve a copy of the template's home with the forge installed, in a fork of its world. */
    static async open(): Promise<TestForge> {
        // copy the served home in a fork of its world
        const { space: home, world } = await TestSpace.copy(FORGE_TEMPLATE, FORGE_WORLD_OPTIONS);
        const owner = home.universe.person(OWNER_EMAIL);
        const { installation, tenants } = world;

        return new TestForge({ home, installation, owner, tenants });
    }

    /** Write the world: the forge, the publishing account and the tenants. */
    static async write(home: TestSpace): Promise<ForgeWorld> {
        // create the publishing account as its owner, the one the machine fixtures publish under
        const owner = await home.universe.signIn(OWNER_EMAIL);
        await TestSpace.account(home.universe, owner);
        // sign in and create the tenants most tests take, and the stranger to them
        const tenants: Identifier<"account">[] = [];
        for (let index = 1; index <= TEMPLATE_TENANTS; index += 1) {
            const created = await TestForge.#createTenant(home, `tenant-${index}`, REGION);
            tenants.push(created.accountId);
        }
        await home.universe.signIn(STRANGER_EMAIL);
        await home.simulation.idle();
        const installation = await TestForge.#installation(home);

        return { installation, tenants };
    }

    /** Read the forge's installation from its revision's binding of the repository storage. */
    static async #installation(home: TestSpace): Promise<Identifier<"installation">> {
        const revisions = await home.database
            .snapshot()
            .read(installationRevision.table)
            .findMany({});
        const bound = revisions.find((revision) =>
            revision.bindings.some(
                (entry) =>
                    entry.packageId === forgePackage.id && entry.name === repositoryStorage.name,
            ),
        );

        return present(bound, "the forge's repository storage binding").installationId;
    }

    /** Call the forge as a person with their client's token, or anonymously. */
    client(person?: TestPerson) {
        return createClient(registryPackage, this.endpoint(person), this.environment);
    }

    /** Address the forge's package as a person with their token, or anonymously. */
    endpoint(person?: TestPerson): {
        readonly url: string;
        readonly fetch: (request: Request) => Promise<Response>;
    } {
        // sign a person's requests with their token, and send anonymous ones as they are
        const server = this.home.server(forgePackage);
        const signed =
            person === undefined
                ? (request: Request) => server.fetch(request)
                : this.home.person(person).endpoint(forgePackage).fetch;

        return { url: ORIGIN, fetch: signed };
    }

    /** Read one of the forge's mounts below its path as a person, or anonymously. */
    fetch(path: string, init: RequestInit = {}, person?: TestPerson): Promise<Response> {
        const { url, fetch } = this.endpoint(person);

        return fetch(new Request(`${url}${path}`, init));
    }

    /** Publish a build as the owner with the builds it runs with, creating its package. */
    async publish(
        reader: BuildReader,
        visibility: Select<typeof packageObject.table>["visibility"] = "private",
        builds: readonly BuildReader[] = [],
    ): Promise<Published> {
        // push its image
        const publisher = this.client(this.owner);
        const { id: packageId, version } = PackageManifest.package(reader.manifest);
        const image = await this.push(reader, this.owner, builds);
        await publisher.package.publish({
            spaceId: this.space,
            requestId: RequestId.create(this.environment),
            accountId: ACCOUNT_ID,
            visibility,
            image,
            commit: TEST_COMMIT,
        });
        const release = await publisher.release.resolve({
            spaceId: this.space,
            parentId: packageId,
            release: version,
        });
        const manifest = await BuildStore.digest(reader.manifest, this.environment.crypto);

        return { reader, packageId, manifest, image, release };
    }

    /** Publish a pushed image of a package as a person, answering the release its tag points at. */
    async release(
        input: {
            readonly parentId: PackageId;
            readonly accountId?: Identifier<"account">;
            readonly image: Digest;
            readonly requestId?: string;
            readonly commit: string;
            readonly tag?: string;
        },
        person: TestPerson = this.owner,
    ): Promise<Release> {
        // publish the image
        const client = this.client(person);
        await client.package.publish({
            spaceId: this.space,
            requestId: input.requestId ?? RequestId.create(this.environment),
            accountId: input.accountId ?? ACCOUNT_ID,
            image: input.image,
            commit: input.commit,
            ...(input.tag === undefined ? {} : { tag: input.tag }),
        });

        return await client.release.resolve({
            spaceId: this.space,
            parentId: input.parentId,
            release: input.tag ?? LATEST_TAG,
        });
    }

    /** Push a stored image as the owner, then publish the account's packages in it under a tag. */
    async publishImage(
        store: BuildStore,
        image: Digest,
        tag: string,
        options: Pick<MethodInput<typeof packageObject, "publish">, "commit"> &
            Partial<Pick<MethodInput<typeof packageObject, "publish">, "tags">>,
    ): Promise<readonly PackageRelease[]> {
        // push the image's objects below the account's handle, then publish once
        await this.builds(ACCOUNT_HANDLE).push(store, [image]);
        const published = await this.client(this.owner).package.publish({
            spaceId: this.space,
            requestId: RequestId.create(this.environment),
            accountId: ACCOUNT_ID,
            image,
            tag,
            ...options,
        });

        return published.releases;
    }

    /** Push a build in an image with the builds it runs with, answering the image's digest. */
    async push(
        reader: BuildReader,
        person: TestPerson = this.owner,
        builds: readonly BuildReader[] = [],
    ): Promise<Digest> {
        // store the builds and make an image of them by package name
        await using store = await TestBuildStore.create(this.environment);
        const root = await store.putIndexed(reader);
        const named: Record<string, Digest> = {};
        for (const build of builds) {
            named[build.manifest.package.name] = await store.putIndexed(build);
        }
        const image = await store.putImage(
            await BuildImage.hoisted(root, named, (digest) => store.get(digest)),
        );

        // push the image's objects below the account's handle
        const uploads = this.builds(PackageName.split(reader.manifest.package.name).handle, person);
        await uploads.push(store, [image]);

        return image;
    }

    /** Connect to the forge's uploads below an account's handle as a person, the owner first. */
    builds(handle: string, person: TestPerson = this.owner): RemoteBuildStore {
        const { url, fetch } = this.endpoint(person);

        return new RemoteBuildStore(`${url}${forgePackage.mounts.builds.path}/${handle}/`, fetch);
    }

    /** Open the store of builds and npm archives in the forge home's blobs. */
    async store(): Promise<BuildStore> {
        return Forge.openStore({
            blobs: (await this.home.store()).blobs,
            environment: this.environment,
            distribution: BuildLayer.empty(),
        });
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

    /** Take the template's next tenant of its region, else create an account of a new owner. */
    async tenant(region: RegionCode = REGION): Promise<Tenant> {
        // take the template's next tenant, restoring its owner and viewer
        this.#tenants += 1;
        const handle = `tenant-${this.#tenants}`;
        const templated = region === REGION ? this.#templated[this.#tenants - 1] : undefined;
        const { universe } = this.home;
        const created =
            templated === undefined
                ? await TestForge.#createTenant(
                      this.home,
                      `tenant-${this.#tenants + TEMPLATE_TENANTS}`,
                      region,
                  )
                : {
                      accountId: templated,
                      owner: universe.person(`${handle}@example.test`),
                      reader: universe.person(`${handle}-reader@example.test`),
                  };
        await this.simulation.idle();

        // host its home with its forge as its owner
        const forge = await this.account(created.accountId, created.owner);

        return { ...created, stranger: this.stranger(), forge };
    }

    /** Open the forge of an account's home as the account's owner, hosting the home once. */
    async account(accountId: Identifier<"account">, owner: TestPerson): Promise<TestForge> {
        const home = await this.home.home(accountId, owner);

        return new TestForge({
            home,
            installation: await TestForge.#installation(home),
            owner,
            tenants: [],
        });
    }

    /** Sign an account's owner and viewer in, and create the account as its owner. */
    static async #createTenant(
        home: TestSpace,
        handle: string,
        region: RegionCode,
    ): Promise<Pick<Tenant, "accountId" | "owner" | "reader">> {
        // sign the owner and the viewer in, and create the account as its owner
        const owner = await home.universe.signIn(`${handle}@example.test`);
        const reader = await home.universe.signIn(`${handle}-reader@example.test`);
        const created = await owner.client(accountPackage).account.create({
            scope: owner.subject.id,
            requestId: RequestId.create(home.environment),
            handle,
            name: "Tenant",
            kind: "shared",
            regionId: region,
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

    /** Enroll a machine of an account as a person, published through the region's relay. */
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
        return this.home.as(subject).client(forgePackage);
    }

    /** Call the registry protocol the forge implements as a subject. */
    asRegistry(subject: Subject) {
        return this.home.as(subject).client(registryPackage, forgePackage);
    }

    /** Open the forge's platform repositories as its bound Git storage keeps them. */
    async storage(): Promise<LocalGitStorage> {
        // take the target its revisions bind the repository storage to
        const revisions = await this.home.database
            .snapshot()
            .read(installationRevision.table)
            .findMany({ where: { installationId: this.installation } });
        const bound = revisions.flatMap((revision) => revision.bindings);
        const target = present(
            bound.find((entry) => entry.name === repositoryStorage.name),
            "the forge's Git storage binding",
        ).target;

        return new LocalGitStorage(join(this.home.resources, target), this.home.environment);
    }

    /** Look up the universe's names, such as the claims of packages' identities. */
    get claims(): ClaimLookup {
        return ClaimLookup.of(this.home.universe.storage.database.snapshot().read(claimTable));
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

        const store = await this.store();

        return await store.contents((await store.image(resolved.image)).root);
    }
}

import { Directory } from "@destack/account/address";
import { BuildObject, BuildStore, PackageClient } from "@destack/build/store";
import { Claim, ClaimLookup, claimTable } from "@destack/object";
import { type PackageId, PackageName, PackageRelease } from "@destack/package";
import { PackageFile } from "@destack/package/file";
import {
    BuildReader,
    Lock,
    type PackageLocation,
    PackageManifest,
} from "@destack/package/manifest";
import { Digest, type Identifier, nth, present, schema } from "@destack/schema";
import type { Fetch } from "@destack/object";
import { createClient } from "@destack/object/client";
import { RegistryError } from "../error/index.ts";
import { packageObject } from "../object/index.ts";
import registryPackage from "../package.ts";
import type { Environment } from "@destack/environment";
import type { Lease } from "@destack/resource";
import { RequestId } from "@destack/sync";
import { TarballEntry } from "@destack/package/archive";
import type { Runtime } from "@destack/package/runtime";

/** The forge a caller calls for a package. */
type ForgeEndpoint = {
    /** The forge's URL, its package's path. */
    readonly url: string;
    /** The space storing the forge's packages, its residency home. */
    readonly space: string;
    /** Fetch from it as the caller. */
    readonly fetch: Fetch;
};

/** The store a publisher pushes and releases locks from. */
type LockStore = Pick<
    BuildStore,
    "closure" | "contents" | "get" | "head" | "lock" | "putLock" | "read"
>;

/** Whose forge a caller calls: a package's publisher's, or an account's. */
type ForgeOwner = { readonly package: PackageId } | { readonly account: Identifier<"account"> };

/** The forges of published releases and repositories, called as one caller. */
export class Registry {
    /** Find the forge of a package's publisher or of an account. */
    readonly #find: (owner: ForgeOwner) => Promise<ForgeEndpoint>;
    /** The environment the forge's clients run in. */
    readonly #environment: Environment;
    /** The releases this caller ships, whose manifests it finds before any forge's. */
    readonly #releases: readonly PackageRelease[];
    /** The reader of each build opened so far, by package and manifest, as builds never change. */
    readonly #readers = new Map<string, Promise<BuildReader>>();

    /** Call the forge of each package's publisher and each account, after the shipped releases. */
    constructor(
        find: (owner: ForgeOwner) => Promise<ForgeEndpoint>,
        environment: Environment,
        releases: readonly PackageRelease[] = [],
    ) {
        this.#find = find;
        this.#environment = environment;
        this.#releases = releases;
    }

    /** Call forges through the universe directory's replica, signing for each forge's home. */
    static of(
        directory: Directory,
        home: (account: Identifier<"account">) => Promise<Identifier<"space">>,
        sign: (audience: PackageId, home: Identifier<"space">) => Fetch,
        environment: Environment,
        installation: string,
        releases: readonly PackageRelease[] = [],
    ): Registry {
        const claims = ClaimLookup.of(directory.snapshot.read(claimTable));

        return new Registry(
            async (owner) => {
                // call the forge below its installation in the home of the owning account's residency
                const accountId = await Registry.#account(claims, owner);
                const spaceId = await home(accountId);
                const now = environment.clock.now();
                const url = await directory.mount(spaceId, installation, registryPackage.id, now);

                return { url, space: spaceId, fetch: sign(registryPackage.id, spaceId) };
            },
            environment,
            releases,
        );
    }

    /** Read the account owning a forge: an account itself, or the one publishing a package. */
    static async #account(claims: ClaimLookup, owner: ForgeOwner): Promise<Identifier<"account">> {
        // take an account itself
        if ("account" in owner) {
            return owner.account;
        }

        // find the account claiming the package's identifier
        const found = await Claim.lookup(packageObject, claims, "id", [owner.package]);
        if (found === undefined) {
            throw RegistryError.of(
                "PUBLISHER_NOT_FOUND",
                `publishing account for '${owner.package}' not found`,
            );
        }

        return schema.identifier("account").parse(found.scope);
    }

    /** Resolve a package's published release of a version, or of the version a tag points at. */
    async resolve(packageId: PackageId, release: string) {
        const forge = await this.#find({ package: packageId });

        return createClient(registryPackage, forge, this.#environment).release.resolve({
            spaceId: forge.space,
            parentId: packageId,
            release,
        });
    }

    /** Lease short-lived access to read a repository at the forge of its account. */
    async lease(
        account: Identifier<"account">,
        repositoryId: Identifier<"repository">,
    ): Promise<Lease> {
        const forge = await this.#find({ account });
        const client = createClient(registryPackage, forge, this.#environment);

        return client.repository.open({ spaceId: forge.space, id: repositoryId, mode: "read" });
    }

    /** Find the digest of the lock of a shipped release, or of a package's published one. */
    async find(packageId: PackageId, release: string): Promise<Digest> {
        // take a shipped release first, a dev build of it among them
        const shipped = PackageRelease.select(this.#releases, packageId, release);
        if (shipped !== undefined) {
            return shipped.lock;
        }

        // resolve the release at its forge
        const { lock } = await this.resolve(packageId, release);

        return lock;
    }

    /** Push a stored lock and the builds it lists to its account's forge, staging it unreleased. */
    async push(
        store: LockStore,
        lock: Digest,
        account: Identifier<"account">,
        signal?: AbortSignal,
    ): Promise<void> {
        // push the lock's objects below the account's handle
        const read = await store.lock(lock);
        const { handle } = PackageName.split(
            PackageManifest.package(await store.get(read.root)).name,
        );
        const forge = await this.#find({ account });
        await Registry.#uploads(forge, handle).push(store, [lock], signal);
    }

    /** Release stored locks' roots under a tag at their account's forge, required ones first. */
    async release(
        store: LockStore,
        locks: readonly Digest[],
        account: Identifier<"account">,
        tag: string,
        commit?: string,
    ): Promise<readonly PackageRelease[]> {
        // read each lock and its root's package
        const read = await Promise.all(locks.map((lock) => store.lock(lock)));
        const roots = new Map<Digest, PackageRelease>();
        for (const [index, each] of read.entries()) {
            const own = PackageManifest.package(await store.get(each.root));
            roots.set(each.root, { lock: nth(locks, index), package: own });
        }
        if (roots.size === 0) {
            return [];
        }

        // push every lock's objects once below the account's handle
        const { handle } = PackageName.split(nth([...roots.values()], 0).package.name);
        const forge = await this.#find({ account });
        await Registry.#uploads(forge, handle).push(store, locks);
        const merged = Lock.of(
            nth(read, 0).root,
            Object.fromEntries(read.flatMap((each) => Object.entries(each.builds))),
        );

        // publish a layer at a time
        const released: PackageRelease[] = [];
        for (const layer of Lock.layers(merged)) {
            const outcomes = await Promise.all(
                layer.flatMap((digest) => {
                    const root = roots.get(digest);

                    return root === undefined
                        ? []
                        : [this.#publish(forge, root, account, tag, commit)];
                }),
            );
            released.push(...outcomes);
        }

        return released;
    }

    /** Release the account's packages in a stored lock under a tag, each under its own lock. */
    async publish(
        store: LockStore,
        lock: Digest,
        account: Identifier<"account">,
        tag: string,
        commit?: string,
    ): Promise<readonly PackageRelease[]> {
        // lock each of the account's Destack builds within the lock
        const read = await store.lock(lock);
        const { handle } = PackageName.split(
            PackageManifest.package(await store.get(read.root)).name,
        );
        const owned: Digest[] = [];
        for (const digest of Lock.builds(read)) {
            const manifest = await store.get(digest);
            if (
                PackageManifest.isPackage(manifest) &&
                PackageName.split(manifest.package.name).handle === handle
            ) {
                owned.push(await store.putLock(Lock.at(read, digest)));
            }
        }

        return await this.release(store, owned, account, tag, commit);
    }

    /** Publish a pushed lock as its package's release, creating the package the first time. */
    async #publish(
        forge: ForgeEndpoint,
        root: PackageRelease,
        account: Identifier<"account">,
        tag: string,
        commit: string | undefined,
    ): Promise<PackageRelease> {
        await createClient(registryPackage, forge, this.#environment).package.publish({
            spaceId: forge.space,
            requestId: RequestId.create(this.#environment),
            packageId: root.package.id,
            accountId: account,
            name: PackageName.split(root.package.name).local,
            lock: root.lock,
            ...(commit === undefined ? {} : { commit }),
            tag,
        });

        return root;
    }

    /** Open the upload endpoint below an account's handle at its forge. */
    static #uploads(forge: ForgeEndpoint, handle: string): PackageClient {
        const uploads = `${forge.url}${registryPackage.mounts.builds.path}/${handle}/`;

        return new PackageClient(uploads, forge.fetch);
    }

    /** Pull a published lock into a store for some runtimes, refusing another package's root. */
    async pull(
        store: Pick<BuildStore, "get" | "lock" | "pull">,
        packageId: PackageId,
        lock: Digest,
        runtimes: readonly Runtime[] | "all",
    ): Promise<Lock> {
        // pull the lock, reaching its package's forge only for what the store lacks
        const root = { kind: "lock", digest: lock } as const;
        const open = () => this.store(packageId, root);
        const source = {
            async *read(objects: readonly Pick<BuildObject, "kind" | "digest">[]) {
                if (objects.length > 0) {
                    yield* (await open()).read(objects);
                }
            },
        };
        await store.pull(source, root, runtimes);
        const pulled = await store.lock(lock);

        // require its root to be a build of the package
        const built = PackageManifest.package(await store.get(pulled.root));
        if (built.id !== packageId) {
            throw RegistryError.of(
                "MANIFEST_INVALID",
                `lock '${lock}' is invalid: its root is a build of '${built.name}' instead of the requested package`,
            );
        }

        return pulled;
    }

    /** Locate the files of a package's published build by its manifest's digest. */
    async locate(
        packageId: PackageId,
        manifest: Digest,
    ): Promise<{
        readonly location: PackageLocation;
        readonly fetch: (input: URL, init: RequestInit) => Promise<Response>;
    }> {
        const forge = await this.#find({ package: packageId });

        return {
            location: {
                manifest,
                url: `${forge.url}${registryPackage.mounts.builds.path}/${manifest}/`,
            },
            fetch: (input, request) => forge.fetch(new Request(input.href, request)),
        };
    }

    /** Open a published lock's or build's objects at its package's forge, as a build store. */
    async store(
        packageId: PackageId,
        root: Pick<BuildObject, "kind" | "digest">,
    ): Promise<PackageClient> {
        const forge = await this.#find({ package: packageId });
        const url = `${forge.url}${registryPackage.mounts.builds.path}/${BuildObject.path(root)}/`;

        return new PackageClient(url, forge.fetch);
    }

    /** Read the build a published build attached at a path, such as its runtime build. */
    async referrer(
        packageId: PackageId,
        manifest: Digest,
        path: string,
    ): Promise<Digest | undefined> {
        // ask the build's forge for the referrer at the path, as OCI's referrers API answers
        const { location, fetch } = await this.locate(packageId, manifest);
        const url = new URL(`referrers/${encodeURIComponent(path)}`, location.url);
        const response = await fetch(url, {});
        if (response.status === 404) {
            await response.body?.cancel();

            return undefined;
        }
        if (!response.ok) {
            await response.body?.cancel();
            throw RegistryError.of(
                "REMOTE_UNREACHABLE",
                `could not read '${path}' of '${manifest}' from the forge: HTTP ${response.status}`,
            );
        }

        return PackageFile.parse(await response.json()).digest;
    }

    /** Open the build of a package's published release by its manifest's digest, file by file. */
    read(packageId: PackageId, manifest: Digest): Promise<BuildReader> {
        // open each build once, forgetting a failed open
        const key = `${packageId} ${manifest}`;
        let reader = this.#readers.get(key);
        if (reader === undefined) {
            reader = this.#open(packageId, manifest);
            this.#readers.set(key, reader);
            reader.catch(() => this.#readers.delete(key));
        }

        return reader;
    }

    /** Open a build the forge serves by its digest, among the builds a package's locks list. */
    async #open(packageId: PackageId, manifest: Digest): Promise<BuildReader> {
        // read the build the forge serves by its manifest's digest
        const { location, fetch } = await this.locate(packageId, manifest);

        return await BuildReader.open(location, fetch, this.#environment.crypto);
    }

    /** The releases the machine's distribution ships, run by scopes that install none of them. */
    get distribution(): readonly PackageRelease[] {
        return this.#releases;
    }

    /** Open the build of a package's published release, by its version or a distribution tag. */
    async open(packageId: PackageId, release: string): Promise<BuildReader> {
        // read the release's lock from its forge, then open its root build
        const digest = await this.find(packageId, release);
        const source = await this.store(packageId, { kind: "lock", digest });
        let bytes: Uint8Array<ArrayBuffer> | undefined;
        for await (const entry of source.read([{ kind: "lock", digest }])) {
            bytes = await TarballEntry.bytes(entry);
        }
        const read = present(bytes, `lock '${digest}'`);
        if ((await Digest.of(read, this.#environment.crypto)) !== digest) {
            throw RegistryError.of(
                "MANIFEST_INVALID",
                `lock '${digest}' is invalid: its bytes do not match its digest`,
            );
        }
        const lock = Lock.parse(JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(read)));

        return this.read(packageId, lock.root);
    }
}

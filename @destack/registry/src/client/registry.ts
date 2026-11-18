import { Directory } from "@destack/account/address";
import { BuildObject, BuildStore, PackageClient } from "@destack/build/store";
import { Claim, ClaimLookup, claimTable } from "@destack/object";
import { type PackageId, PackageName, PackageRelease } from "@destack/package";
import { PackageFile } from "@destack/package/file";
import {
    BuildReader,
    BuildImage,
    type PackageLocation,
    PackageManifest,
} from "@destack/package/manifest";
import { Commit, Digest, type Identifier, nth, present, schema } from "@destack/schema";
import type { Fetch } from "@destack/object";
import { type ClientOf, createClient } from "@destack/object/client";
import { Failure } from "@destack/error";
import { RegistryError } from "../error/index.ts";
import {
    packageObject,
    reference,
    release,
    repository,
    tag,
    type PackageVisibility,
} from "../object/index.ts";
import registryPackage from "../package.ts";
import type { Environment } from "@destack/environment";
import type { Lease } from "@destack/resource";
import { type ObjectReference, RequestId } from "@destack/sync";
import type { InstallationSelection } from "@destack/object";
import { TarballEntry } from "@destack/package/archive";
import type { Runtime } from "@destack/package/runtime";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";

/** The forge a caller calls for a package. */
type ForgeEndpoint = {
    /** The forge's URL, its package's path. */
    readonly url: string;
    /** The space storing the forge's packages, its residency home. */
    readonly space: string;
    /** Fetch from it as the caller. */
    readonly fetch: Fetch;
};

/** The build store a publish reads images and their builds from. */
export type PublishStore = Pick<
    BuildStore,
    | "closure"
    | "contents"
    | "get"
    | "head"
    | "image"
    | "putImage"
    | "read"
    | "attachments"
    | "environment"
>;

/** Whose forge a caller calls: a package's publisher's, or an account's. */
type ForgeOwner =
    | { readonly package: PackageId }
    | { readonly repository: Identifier<"repository"> }
    | { readonly account: Identifier<"account"> }
    | { readonly handle: string }
    | { readonly home: Identifier<"space"> };

/** The package file every build carries, whose `publishConfig` asks its access. */
const PACKAGE_FILE = "package.json";

/** The visibility each of npm's access levels publishes a package at. */
const NPM_ACCESS = { public: "public", restricted: "private" } as const satisfies Record<
    string,
    PackageVisibility
>;

/** The part of a package file asking its access, as npm reads it at publish. */
const PackageAccess = schema.looseObject({
    /** The settings npm publishes with. */
    publishConfig: schema
        .looseObject({
            /** Whether the package is public or restricted to its owners. */
            access: schema.enum(["public", "restricted"]).exactOptional(),
        })
        .exactOptional(),
});

/** How a release publishes its packages: its commit, and who reads a package it creates. */
export interface PublishOptions {
    /** The commit the roots compiled, absent for a prerelease of uncommitted changes. */
    readonly commit?: string;
    /** Who can read a package the release creates, else as its package file asks, else unlisted. */
    readonly visibility?: PackageVisibility;
    /** The further distribution tags to point at each released version, as npm's dist-tag add. */
    readonly tags?: readonly string[];
}

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
    /** The registry's spans and durations. */
    readonly #scope: TelemetryScope;

    /** Call the forge of each package's publisher and each account, after the shipped releases. */
    constructor(
        find: (owner: ForgeOwner) => Promise<ForgeEndpoint>,
        environment: Environment,
        releases: readonly PackageRelease[] = [],
    ) {
        // take the forges, the environment and the shipped releases, and trace on its telemetry
        this.#find = find;
        this.#environment = environment;
        this.#releases = releases;
        this.#scope = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
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
                // call the forge below its installation in its home, or in the owning account's residency home
                const spaceId =
                    "home" in owner
                        ? owner.home
                        : await home(await Registry.#account(directory, claims, owner));
                const now = environment.clock.now();
                const url = await directory.mount(spaceId, installation, registryPackage.id, now);

                return { url, space: spaceId, fetch: sign(registryPackage.id, spaceId) };
            },
            environment,
            releases,
        );
    }

    /** Read the account owning a forge: an account itself, or the one publishing a package. */
    static async #account(
        directory: Directory,
        claims: ClaimLookup,
        owner: Exclude<ForgeOwner, { readonly home: unknown }>,
    ): Promise<Identifier<"account">> {
        // take an account itself
        if ("account" in owner) {
            return owner.account;
        }
        // find the account claiming a handle
        else if ("handle" in owner) {
            const claimed = await directory.account(owner.handle);
            if (claimed === undefined) {
                throw RegistryError.of(
                    "PUBLISHER_NOT_FOUND",
                    `owning account for '@${owner.handle}' not found`,
                );
            }

            return claimed;
        }

        // find the account claiming the repository's or the package's identifier
        const found =
            "repository" in owner
                ? await Claim.lookup(repository, claims, "id", [owner.repository])
                : await Claim.lookup(packageObject, claims, "id", [owner.package]);
        if (found === undefined) {
            throw RegistryError.of(
                "PUBLISHER_NOT_FOUND",
                `owning account for '${"repository" in owner ? owner.repository : owner.package}' not found`,
            );
        }

        return schema.identifier("account").parse(found.scope);
    }

    /** Read the address npm clients ask for a handle's packages: its forge's npm mount. */
    async npm(handle: string): Promise<string> {
        const forge = await this.#find({ handle });

        return `${forge.url}${registryPackage.mounts.npm.path}`;
    }

    /** Resolve a package's published release of a version, or of the version a tag points at. */
    async resolve(packageId: PackageId, named: string) {
        const forge = await this.#find({ package: packageId });

        return createClient(registryPackage, forge, this.#environment).release.resolve({
            spaceId: forge.space,
            parentId: packageId,
            release: named,
        });
    }

    /** Read the version a followed release or tag names at its package's forge. */
    async version(packageId: PackageId, followed: ObjectReference): Promise<string> {
        // read the release itself, or the tag, by its identifier
        const forge = await this.#find({ package: packageId });
        const client = createClient(registryPackage, forge, this.#environment);
        const row =
            followed.type === tag.name
                ? await client.tag.get({
                      spaceId: forge.space,
                      id: schema.identifier("tag").parse(followed.id),
                  })
                : await client.release.get({
                      spaceId: forge.space,
                      id: schema.identifier("release").parse(followed.id),
                  });

        return row.version;
    }

    /** Read the release, tag or repository reference a selection names, at its forge's home. */
    async followed(
        packageId: PackageId,
        selection: Exclude<InstallationSelection, { readonly kind: "host" | "checkout" }>,
    ): Promise<ObjectReference> {
        // take the release a version names
        if (selection.kind === "release") {
            const found = await this.resolve(packageId, selection.version);

            return release.reference(found.scope, found.id);
        }

        // find the tag of the package, or the reference of the repository, by its name
        const forge =
            selection.kind === "tag"
                ? await this.#find({ package: packageId })
                : await this.#find({ repository: selection.repository });
        const client = createClient(registryPackage, forge, this.#environment);
        const { items } =
            selection.kind === "tag"
                ? await client.tag.list({
                      spaceId: forge.space,
                      where: { parentId: packageId, name: selection.tag },
                      limit: 1,
                  })
                : await client.reference.list({
                      spaceId: forge.space,
                      where: { parentId: selection.repository, name: selection.reference },
                      limit: 1,
                  });
        const [row] = items;
        if (row === undefined) {
            throw RegistryError.of(
                "RELEASE_NOT_FOUND",
                selection.kind === "tag"
                    ? `tag '${selection.tag}' of '${packageId}' not found`
                    : `reference '${selection.reference}' of repository '${selection.repository}' not found`,
            );
        }

        return selection.kind === "tag"
            ? tag.reference(row.scope, row.id)
            : reference.reference(row.scope, row.id);
    }

    /** Read a followed reference at its forge's home: its repository, name and commit. */
    async reference(followed: ObjectReference): Promise<{
        readonly repository: Identifier<"repository">;
        readonly name: string;
        readonly commit: Commit | undefined;
    }> {
        // read the reference by its identifier at its home
        const forge = await this.#find({ home: schema.identifier("space").parse(followed.scope) });
        const client = createClient(registryPackage, forge, this.#environment);
        const row = await client.reference.get({
            spaceId: forge.space,
            id: schema.identifier("reference").parse(followed.id),
        });

        // read its commit, absent before its first push
        const commit = row.commit === null ? undefined : Commit.parse(row.commit);

        return { repository: row.parentId, name: row.name, commit };
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

    /** Find the digest of the image of a shipped release, or of a package's published one. */
    async find(packageId: PackageId, named: string): Promise<Digest> {
        // take a shipped release first, a dev build of it among them
        const shipped = PackageRelease.select(this.#releases, packageId, named);
        if (shipped !== undefined) {
            return shipped.image;
        }

        // resolve the release at its forge
        const { image } = await this.resolve(packageId, named);

        return image;
    }

    /** Push a stored image and its builds to its account's forge, staging it unreleased. */
    async push(
        store: PublishStore,
        image: Digest,
        account: Identifier<"account">,
        signal?: AbortSignal,
    ): Promise<void> {
        return this.#scope.span(
            "destack.registry.push",
            { "destack.account.id": account },
            async () => {
                // push the image's objects below the account's handle
                const read = await store.image(image);
                const { handle } = PackageName.split(
                    PackageManifest.package(await store.get(read.root)).name,
                );
                const forge = await this.#find({ account });
                await Registry.#uploads(forge, handle).push(store, [image], signal);
            },
        );
    }

    /** Release stored images' roots under a tag at their account's forge, required ones first. */
    async release(
        store: PublishStore,
        images: readonly Digest[],
        account: Identifier<"account">,
        tagName: string,
        options: PublishOptions = {},
    ): Promise<readonly PackageRelease[]> {
        return this.#scope.span(
            "destack.registry.release",
            { "destack.account.id": account },
            async () => {
                // read each image and its root's package
                const read = await Promise.all(images.map((image) => store.image(image)));
                const roots = new Map<Digest, PackageRelease>();
                for (const [index, each] of read.entries()) {
                    const own = PackageManifest.package(await store.get(each.root));
                    roots.set(each.root, { image: nth(images, index), package: own });
                }
                if (roots.size === 0) {
                    return [];
                }

                // order the roots by dependency layer, each after the ones it requires
                const merged = BuildImage.of(
                    nth(read, 0).root,
                    Object.fromEntries(read.flatMap((each) => Object.entries(each.builds))),
                );
                const layers = BuildImage.layers(merged).map((layer) =>
                    layer.flatMap((digest) => {
                        const root = roots.get(digest);

                        return root === undefined ? [] : [root];
                    }),
                );

                // leave the roots the tag already points at, as publishing them again changes nothing
                const forge = await this.#find({ account });
                const client = createClient(registryPackage, forge, this.#environment);
                const tagged = await Promise.all(
                    [...roots.values()].map((root) =>
                        Registry.#tagged(client, forge, root, tagName),
                    ),
                );
                const pending = new Set(
                    [...roots.values()]
                        .filter((root, index) => tagged[index] !== root.image)
                        .map((root) => root.image),
                );

                // push the pending images' objects once below the account's handle
                if (pending.size > 0) {
                    const { handle } = PackageName.split(nth([...roots.values()], 0).package.name);
                    await Registry.#uploads(forge, handle).push(store, [...pending]);

                    // publish them a layer at a time, each as its package file asks
                    for (const layer of layers) {
                        await Promise.all(
                            layer
                                .filter((root) => pending.has(root.image))
                                .map(async (root) => {
                                    // take the visibility asked, else the package file's
                                    const visibility =
                                        options.visibility ??
                                        (await Registry.#visibility(store, root));

                                    return this.#publish(
                                        forge,
                                        root,
                                        account,
                                        tagName,
                                        visibility === undefined
                                            ? options
                                            : { ...options, visibility },
                                    );
                                }),
                        );
                    }
                }

                // point each further tag at every root's version, as npm's dist-tag add does
                for (const name of options.tags ?? []) {
                    await Promise.all(
                        [...roots.values()].map((root) => this.#point(client, forge, root, name)),
                    );
                }

                return layers.flat();
            },
        );
    }

    /** Release the account's packages in a stored image under a tag, each under its own image. */
    async publish(
        store: PublishStore,
        image: Digest,
        account: Identifier<"account">,
        tagName: string,
        options: PublishOptions = {},
    ): Promise<readonly PackageRelease[]> {
        return this.#scope.span(
            "destack.registry.publish",
            { "destack.account.id": account },
            async () => {
                // make an image of each of the account's Destack builds within the image
                const read = await store.image(image);
                const { handle } = PackageName.split(
                    PackageManifest.package(await store.get(read.root)).name,
                );
                const owned: Digest[] = [];
                for (const digest of BuildImage.builds(read)) {
                    const manifest = await store.get(digest);
                    if (
                        PackageManifest.isPackage(manifest) &&
                        PackageName.split(manifest.package.name).handle === handle
                    ) {
                        owned.push(await store.putImage(BuildImage.at(read, digest)));
                    }
                }

                return await this.release(store, owned, account, tagName, options);
            },
        );
    }

    /** Publish a pushed image as its package's release, creating the package the first time. */
    async #publish(
        forge: ForgeEndpoint,
        root: PackageRelease,
        account: Identifier<"account">,
        tagName: string,
        options: PublishOptions,
    ): Promise<PackageRelease> {
        await createClient(registryPackage, forge, this.#environment).package.publish({
            spaceId: forge.space,
            requestId: RequestId.create(this.#environment),
            packageId: root.package.id,
            accountId: account,
            name: PackageName.split(root.package.name).local,
            image: root.image,
            ...(options.commit === undefined ? {} : { commit: options.commit }),
            ...(options.visibility === undefined ? {} : { visibility: options.visibility }),
            tag: tagName,
        });

        return root;
    }

    /** Read the visibility a root's package file asks through npm's `publishConfig.access`. */
    static async #visibility(
        store: PublishStore,
        root: PackageRelease,
    ): Promise<PackageVisibility | undefined> {
        // read the package file of the image's root
        const reader = await store.contents((await store.image(root.image)).root);
        const authored = PackageAccess.parse(
            JSON.parse(new TextDecoder().decode(await reader.load(PACKAGE_FILE))),
        );

        // publish public access listed, restricted access private
        return authored.publishConfig?.access === undefined
            ? undefined
            : NPM_ACCESS[authored.publishConfig.access];
    }

    /** Point a package's tag at a root's version unless it points there already. */
    async #point(
        client: ClientOf<typeof registryPackage>,
        forge: ForgeEndpoint,
        root: PackageRelease,
        name: string,
    ): Promise<void> {
        // leave a tag already at the root's image
        if ((await Registry.#tagged(client, forge, root, name)) === root.image) {
            return;
        }

        // point the tag at the root's version
        await client.package.point({
            spaceId: forge.space,
            id: root.package.id,
            requestId: RequestId.create(this.#environment),
            name,
            version: root.package.version,
        });
    }

    /** Read the image a package's tag points at in its forge, none before its first publish. */
    static async #tagged(
        client: ClientOf<typeof registryPackage>,
        forge: ForgeEndpoint,
        root: PackageRelease,
        tagName: string,
    ): Promise<Digest | undefined> {
        try {
            const named = await client.release.resolve({
                spaceId: forge.space,
                parentId: root.package.id,
                release: tagName,
            });

            return named.image;
        } catch (error) {
            // publish a package the caller cannot see yet, whose publish then decides
            const { code } = Failure.of(error);
            if (code === "NOT_FOUND" || code === "FORBIDDEN") {
                return undefined;
            }
            throw error;
        }
    }

    /** Open the upload endpoint below an account's handle at its forge. */
    static #uploads(forge: ForgeEndpoint, handle: string): PackageClient {
        const uploads = `${forge.url}${registryPackage.mounts.builds.path}/${handle}/`;

        return new PackageClient(uploads, forge.fetch);
    }

    /** Pull a published image into a store for some runtimes, refusing another package's root. */
    async pull(
        store: Pick<BuildStore, "get" | "image" | "pull">,
        packageId: PackageId,
        image: Digest,
        runtimes: readonly Runtime[] | "all",
    ): Promise<BuildImage> {
        return this.#scope.span(
            "destack.registry.pull",
            { "destack.package.id": packageId },
            async () => {
                // pull the image, reaching its package's forge only for what the store lacks
                const root = { kind: "image", digest: image } as const;
                const open = () => this.store(packageId, root);
                const source = {
                    async *read(objects: readonly Pick<BuildObject, "kind" | "digest">[]) {
                        if (objects.length > 0) {
                            yield* (await open()).read(objects);
                        }
                    },
                };
                await store.pull(source, root, runtimes);
                const pulled = await store.image(image);

                // require its root to be a build of the package
                const built = PackageManifest.package(await store.get(pulled.root));
                if (built.id !== packageId) {
                    throw RegistryError.of(
                        "MANIFEST_INVALID",
                        `image '${image}' is invalid: its root is a build of '${built.name}' instead of the requested package`,
                    );
                }

                return pulled;
            },
        );
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

    /** Open a published image's or build's objects at its package's forge, as a build store. */
    async store(
        packageId: PackageId,
        root: Pick<BuildObject, "kind" | "digest">,
    ): Promise<PackageClient> {
        const forge = await this.#find({ package: packageId });
        const url = `${forge.url}${registryPackage.mounts.builds.path}/${BuildObject.path(root)}/`;

        return new PackageClient(url, forge.fetch);
    }

    /** List the files a package's published build attached, as OCI's referrers API lists them. */
    async attachments(packageId: PackageId, manifest: Digest): Promise<PackageFile[]> {
        // ask the build's forge for its attachments
        const { location, fetch } = await this.locate(packageId, manifest);
        const response = await fetch(new URL("attachments/", location.url), {});
        if (!response.ok) {
            await response.body?.cancel();
            throw RegistryError.of(
                "REMOTE_UNREACHABLE",
                `could not list the attachments of '${manifest}' from the forge: HTTP ${response.status}`,
            );
        }

        return schema.array(PackageFile).parse(await response.json());
    }

    /** Read the build a published build attached at a path, such as its runtime build. */
    async attachment(
        packageId: PackageId,
        manifest: Digest,
        path: string,
    ): Promise<Digest | undefined> {
        // ask the build's forge for the attachment at the path, as OCI's referrers API answers
        const { location, fetch } = await this.locate(packageId, manifest);
        const url = new URL(`attachments/${encodeURIComponent(path)}`, location.url);
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

    /** Open a build the forge serves by its digest, among the builds a package's images list. */
    #open(packageId: PackageId, manifest: Digest): Promise<BuildReader> {
        return this.#scope.span(
            "destack.registry.read",
            { "destack.package.id": packageId },
            async () => {
                // read the build the forge serves by its manifest's digest
                const { location, fetch } = await this.locate(packageId, manifest);

                return await BuildReader.open(location, fetch, this.#environment.crypto);
            },
        );
    }

    /** The releases the machine's distribution ships, run by scopes that install none of them. */
    get distribution(): readonly PackageRelease[] {
        return this.#releases;
    }

    /** Open the build of a package's published release, by its version or a distribution tag. */
    open(packageId: PackageId, named: string): Promise<BuildReader> {
        return this.#scope.span(
            "destack.registry.open",
            { "destack.package.id": packageId },
            async () => {
                // read the release's image from its forge, then open its root build
                const digest = await this.find(packageId, named);
                const source = await this.store(packageId, { kind: "image", digest });
                let bytes: Uint8Array<ArrayBuffer> | undefined;
                for await (const entry of source.read([{ kind: "image", digest }])) {
                    bytes = await TarballEntry.bytes(entry);
                }
                const read = present(bytes, `image '${digest}'`);
                if ((await Digest.of(read, this.#environment.crypto)) !== digest) {
                    throw RegistryError.of(
                        "MANIFEST_INVALID",
                        `image '${digest}' is invalid: its bytes do not match its digest`,
                    );
                }
                const text = new TextDecoder("utf-8", { fatal: true }).decode(read);
                const image = BuildImage.parse(JSON.parse(text));

                return await this.read(packageId, image.root);
            },
        );
    }
}

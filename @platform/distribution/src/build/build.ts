import { TelemetryScope } from "@destack/telemetry";
import { join } from "node:path";
import { account, type StackRelease } from "@destack/account/object";
import type { Session } from "@destack/account/client";
import type { PlatformDefinition } from "@destack/account/server";
import * as stacks from "@platform/stack";
import { RESIDENCIES } from "@platform/stack";
import { Claim, PackageMount, SERVER_EXPORT, SpaceMount } from "@destack/object";
import { InstallationDefinition } from "@destack/stack/declare";
import {
    CompilerPool,
    LocalBuildStore,
    Workspace,
    type WorkspaceUnitOptions,
} from "@destack/build";
import { COMPUTER_RUNTIMES } from "@destack/machine/computer";
import { CLOUD_RUNTIMES } from "@destack/machine/cloudflare";
import type { Clock, Environment } from "@destack/environment";
import {
    DISTRIBUTION_LAYER,
    DISTRIBUTION_MANIFEST,
    DistributionManifest,
    DistributionPlatform,
    PackageName,
} from "@destack/package";
import { type BuildReader, LAYER_INDEX, PackageManifest } from "@destack/package/manifest";
import { Digest, found, present, schema } from "@destack/schema";
import { RemoteBuildStore } from "@destack/build/store";
import { createClient } from "@destack/object/client";
import type { MethodInput } from "@destack/object";
import type { packageObject } from "@destack/registry/object";
import registryPackage from "@destack/registry/package";
import { RequestId } from "@destack/sync";
import { Toolchain } from "@destack/check/toolchain";
import { releaseVersion } from "../distribution/distribution.ts";

/** The prefix of a layer written beside the asset layers before it moves under its digest. */
const ASSETS_STAGING = ".staging";

/** The package declaring the homes' stacks, by its directory below the checkout's root. */
const STACK = "@platform/stack";

/** The export declaring the stack of each kind of space people create. */
const EXPORTS = {
    home: "HOME_STACK",
    space: "SPACE_STACK",
} as const satisfies Record<keyof PlatformDefinition["stacks"], keyof typeof stacks>;

/** The exports declaring each residency home's stack, the same on every universe. */
const STACK_EXPORTS = RESIDENCIES.map(
    (residency) => residency.stack,
) satisfies readonly (keyof typeof stacks)[];

/** A stack definition's installations, by name. */
const INSTALLATIONS = schema.record(schema.string(), InstallationDefinition);

/** The desktop package, whose workspace builds the release of the program computers run. */
const DESKTOP = "@destack/desktop";

/**
 * How long a home may take to serve its forge, in milliseconds.
 *
 * A minute for the machine's directory to sync, and half a minute for the home to start.
 */
const SERVE_MILLISECONDS = 90_000;

/** The status a machine answers to move a request to the machine serving its space, RFC 9110. */
const PLACED_STATUS = 307;

/** The status a starting machine answers with its Retry-After, RFC 9110. */
const UNAVAILABLE_STATUS = 503;

/** The checkout's store of first-party releases, which every universe and target shares. */
export const RELEASES_DIRECTORY = "target/releases";

/** How a build publishes its packages: their commit, visibility and further distribution tags. */
export type PublishOptions = Pick<
    MethodInput<typeof packageObject, "publish">,
    "commit" | "visibility" | "tags"
>;

/** The first-party releases a universe's homes run and its forge publishes. */
export class PlatformBuild implements AsyncDisposable {
    /** The store keeping every build and image, locked to this process until disposed. */
    readonly store: LocalBuildStore;
    /** The releases of the stacks' closure and the published packages, by package name. */
    readonly releases: ReadonlyMap<string, BuildReader>;
    /** The image of each release: its build and every build it runs with, by package name. */
    readonly images: ReadonlyMap<string, Digest>;
    /** The version every release has. */
    readonly version: string;

    /** Keep built releases and their images. */
    private constructor(
        store: LocalBuildStore,
        releases: ReadonlyMap<string, BuildReader>,
        images: ReadonlyMap<string, Digest>,
        version: string,
    ) {
        // keep the store, the releases, their images and their version
        this.store = store;
        this.releases = releases;
        this.images = images;
        this.version = version;
    }

    /** Build the stacks' closure and a checkout's published packages into its store. */
    static async build(
        root: string,
        host: Environment<"files" | "process" | "host">,
        options: Pick<WorkspaceUnitOptions, "isPrerelease"> = {},
    ): Promise<PlatformBuild> {
        // build in a span, which records the build once it succeeds
        return await TelemetryScope.of(import.meta.destack.package, host.telemetry).span(
            "platform.release.build",
            {},
            () => PlatformBuild.#build(root, host, options),
            { level: "info" },
        );
    }

    /** Build the stacks' closure and a checkout's published packages into its store, unrecorded. */
    static async #build(
        root: string,
        host: Environment<"files" | "process" | "host">,
        options: Pick<WorkspaceUnitOptions, "isPrerelease">,
    ): Promise<PlatformBuild> {
        // open the store and the compilers, closing the store when a build fails
        await using opened = new AsyncDisposableStack();
        const store = opened.use(await LocalBuildStore.open(join(root, RELEASES_DIRECTORY), host));
        await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());

        // read the workspace once at the release's version, and the packages of the stacks and of the desktop
        const version = releaseVersion(host);
        const workspace = await Workspace.open(root, compilers, { version });
        const packages = [
            await workspace.package(join(root, STACK)),
            await workspace.package(join(root, DESKTOP)),
        ];

        // build and make an image of each unit, a unit both packages share once, the image attaching its graph
        const releases = new Map<string, BuildReader>();
        const images = new Map<string, Digest>();
        for (const unitPackage of packages) {
            for (const [name, { build, image }] of await unitPackage.imageUnits(options)) {
                releases.set(name, await store.contents(build.manifest));
                images.set(name, image);
            }
        }

        // hand the store to the build, which closes it
        opened.move();

        return new PlatformBuild(store, releases, images, version);
    }

    /** Publish each image under a tag into its account's forge, after the handles it requires. */
    async publish(
        session: Session,
        universe: string,
        tag: string,
        options: PublishOptions = {},
    ): Promise<void> {
        // publish in a span, which records the releases once they succeed
        const { telemetry, clock, network } = this.store.environment;
        const accounts = session.accounts();
        await TelemetryScope.of(import.meta.destack.package, telemetry).span(
            "platform.release.publish",
            { "platform.release.tag": tag },
            async () => {
                // publish each handle's images into its account's forge, after the handles they require
                for (const { handle, names } of PlatformBuild.#runs(this.releases)) {
                    // find the handle's account and its residency's home, which serves its forge
                    const claimed = await accounts.claim.owner(
                        Claim.key(account, "handle", [handle]),
                    );
                    if (claimed === null) {
                        throw new Error(`no account claims '@${handle}'`);
                    }
                    const accountId = account.identifier(claimed.objectId);
                    const home = schema
                        .identifier("space")
                        .parse(await accounts.residency.locate({ account: accountId }));

                    // call the forge at the machine serving the home, as the session bound to the home
                    const send = PlatformBuild.#retry((request) => network.fetch(request), clock);
                    const mount = await PlatformBuild.#locate(SpaceMount.url(universe, home), send);
                    const url = `${mount}${PackageMount.path(registryPackage.id)}`;
                    const fetch = PlatformBuild.#retry(
                        session.fetch(registryPackage.id, { partition: home }),
                        clock,
                    );

                    // find the handle's images its forge has yet to release
                    const registry = createClient(
                        registryPackage,
                        { url, fetch },
                        this.store.environment,
                    );
                    const { items } = await registry.release.list({
                        spaceId: home,
                        where: { image: { in: names.map((name) => found(this.images, name)) } },
                        limit: names.length,
                    });
                    const released = new Set(items.map((item) => item.image));
                    const unreleased = names.filter(
                        (name) => !released.has(found(this.images, name)),
                    );

                    // push those images once below the handle
                    const images = unreleased.map((name) => found(this.images, name));
                    const builds = `${url}${registryPackage.mounts.builds.path}/${handle}/`;
                    await new RemoteBuildStore(builds, fetch).push(this.store, images);

                    // publish them layer by layer, each layer's at once
                    for (const layer of PlatformBuild.#layers(unreleased, this.releases)) {
                        await Promise.all(
                            layer.map((name) =>
                                registry.package.publish({
                                    spaceId: home,
                                    requestId: RequestId.create(this.store.environment),
                                    accountId,
                                    image: found(this.images, name),
                                    tag,
                                    ...options,
                                }),
                            ),
                        );
                    }

                    // point the tags at the images released before, as a push of a pushed image moves its tag
                    const pointing = [tag, ...(options.tags ?? [])];
                    const { items: tagged } = await registry.tag.list({
                        spaceId: home,
                        where: { parentId: { in: items.map((item) => item.parentId) } },
                    });
                    const points = items.flatMap((item) =>
                        pointing
                            .filter(
                                (name) =>
                                    !tagged.some(
                                        (each) =>
                                            each.parentId === item.parentId &&
                                            each.name === name &&
                                            each.version === item.version,
                                    ),
                            )
                            .map((name) => ({ item, name })),
                    );
                    await Promise.all(
                        points.map(({ item, name }) =>
                            registry.package.point({
                                spaceId: home,
                                id: item.parentId,
                                requestId: RequestId.create(this.store.environment),
                                name,
                                version: item.version,
                            }),
                        ),
                    );
                }
            },
            { level: "info" },
        );
    }

    /** Locate a space's mount at the machine serving it, following its address's move once. */
    static async #locate(
        address: string,
        send: (request: Request) => Promise<Response>,
    ): Promise<string> {
        // ask at the address, keeping a move to the machine serving the space
        const response = await send(new Request(address, { redirect: "manual" }));
        await response.body?.cancel();
        const location = response.headers.get("location");

        // take the moved address
        if (response.status === PLACED_STATUS && location !== null) {
            return new URL(location, address).href;
        }
        // refuse a failing space
        else if (response.status >= 500) {
            throw new Error(`space '${address}' answered '${response.status}'`);
        }
        // take the address, which serves the space here
        else {
            return address;
        }
    }

    /** Wrap a fetch to send again as a 503's Retry-After says, until a deadline. */
    static #retry(
        fetch: (request: Request) => Promise<Response>,
        clock: Clock,
    ): (request: Request) => Promise<Response> {
        return async (request) => {
            // read the body once, to send it again after each pause
            const { url, method, headers, redirect } = request;
            const body = request.body === null ? null : await request.arrayBuffer();
            const send = () => fetch(new Request(url, { method, headers, body, redirect }));

            // send again after the pause each 503 asks for, until the deadline
            const deadline = clock.now() + SERVE_MILLISECONDS;
            let response = await send();
            let pause = response.headers.get("retry-after");
            while (
                response.status === UNAVAILABLE_STATUS &&
                pause !== null &&
                clock.now() < deadline
            ) {
                await response.body?.cancel();
                await clock.sleep(Number(pause) * 1000);
                response = await send();
                pause = response.headers.get("retry-after");
            }

            return response;
        };
    }

    /** Group releases by handle, each handle after the handles its releases require. */
    static #runs(
        releases: ReadonlyMap<string, BuildReader>,
    ): { readonly handle: string; readonly names: readonly string[] }[] {
        // group the names by handle, with the other handles each handle's releases require
        const names = new Map<string, string[]>();
        const requires = new Map<string, Set<string>>();
        for (const name of releases.keys()) {
            const { handle } = PackageName.split(name);
            const required = Object.keys(found(releases, name).manifest.requires)
                .filter((dependency) => releases.has(dependency))
                .map((dependency) => PackageName.split(dependency).handle)
                .filter((other) => other !== handle);
            const named = names.get(handle) ?? [];
            const needs = requires.get(handle) ?? new Set<string>();
            named.push(name);
            for (const other of required) {
                needs.add(other);
            }
            names.set(handle, named);
            requires.set(handle, needs);
        }

        // visit each handle's required handles before it, refusing handles that require each other
        const runs: { readonly handle: string; readonly names: readonly string[] }[] = [];
        const visited = new Set<string>();
        const visiting = new Set<string>();
        const visit = (handle: string) => {
            // refuse a handle met again on its own path
            if (visiting.has(handle)) {
                throw new Error(`releases of '@${handle}' require releases that require them back`);
            }
            // add a handle met for the first time after the handles it requires
            else if (!visited.has(handle)) {
                visiting.add(handle);
                for (const other of requires.get(handle) ?? []) {
                    visit(other);
                }
                visiting.delete(handle);
                visited.add(handle);
                runs.push({ handle, names: found(names, handle) });
            }
        };
        for (const handle of names.keys()) {
            visit(handle);
        }

        return runs;
    }

    /** Order a handle's releases into layers, each after the layers holding what it requires. */
    static #layers(
        names: readonly string[],
        releases: ReadonlyMap<string, BuildReader>,
    ): string[][] {
        // read each release's requirements among the handle's releases
        const pending = new Map(
            names.map((name) => [
                name,
                new Set(
                    Object.keys(found(releases, name).manifest.requires).filter(
                        (required) => required !== name && names.includes(required),
                    ),
                ),
            ]),
        );

        // take every release whose requirements earlier layers hold, refusing a cycle
        const layers: string[][] = [];
        while (pending.size > 0) {
            const layer = [...pending]
                .filter(([, required]) => required.size === 0)
                .map(([name]) => name);
            if (layer.length === 0) {
                throw new Error(
                    `releases '${[...pending.keys()].join("', '")}' require each other`,
                );
            }
            layers.push(layer);
            for (const name of layer) {
                pending.delete(name);
            }
            for (const required of pending.values()) {
                for (const name of layer) {
                    required.delete(name);
                }
            }
        }

        return layers;
    }

    /** Close the builds' store, releasing its lock. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.store[Symbol.asyncDispose]();
    }

    /** Every release the build keeps, which machines ship and seed their pull caches with. */
    get readers(): readonly BuildReader[] {
        return [...this.releases.values()];
    }

    /** List every release the build keeps with its role and its image on a platform. */
    distribution(platform: DistributionPlatform): DistributionManifest {
        const releases = [...this.releases].map(([name, reader]) => ({
            package: PackageManifest.package(reader.manifest),
            role: reader.manifest.role,
            images: { [platform]: found(this.images, name) },
        }));

        return { releases };
    }

    /** Map each shipped package a machine runs in its process to its server's specifier. */
    serverBundle(): Readonly<Record<string, string>> {
        // take the packages the stacks install, each on the host or as a builtin machines link
        const declared = [...Object.values(EXPORTS), ...STACK_EXPORTS].map((name) => stacks[name]);
        const installations = declared.flatMap((stack) =>
            Object.values(INSTALLATIONS.parse(stack.definition["installations"] ?? {})),
        );
        const inProcess = new Set(
            installations.filter((each) => each.isolation === "host").map((each) => each.packageId),
        );
        const installed = new Set(installations.map((each) => each.packageId));

        // map each such release exporting a server, refusing one installed on the host without one
        const bundled: Record<string, string> = {};
        for (const { manifest } of this.readers) {
            const { id, name } = PackageManifest.package(manifest);
            const isServing = Object.values(manifest.outputs).some(
                (output) => output.exports[SERVER_EXPORT] !== undefined,
            );
            if (inProcess.has(id) && !isServing) {
                throw new Error(`'${name}' is installed on the host but exports no server`);
            } else if (
                isServing &&
                (inProcess.has(id) || (installed.has(id) && manifest.role === "builtin"))
            ) {
                bundled[name] = `${name}${SERVER_EXPORT.slice(1)}`;
            }
        }

        return bundled;
    }

    /** Write the releases cloud Workers run as assets once per layer, answering its directory. */
    async writeAssets(directory: string): Promise<string> {
        // write the layer beside the earlier ones, which a running Worker may watch
        const { files, crypto, process } = this.store.environment;
        const staging = join(directory, `${ASSETS_STAGING}.${process.id}`);
        await this.store.writeLayer([...this.images.values()], CLOUD_RUNTIMES, staging, "packed");

        // keep it under its index's digest, so a written layer never changes under a watcher
        const index = await files.readTextFile(join(staging, LAYER_INDEX));
        const layer = join(directory, await Digest.of(index, crypto));
        if (await files.exists(layer)) {
            await files.remove(staging, { recursive: true });
        } else {
            await files.rename(staging, layer);
        }

        // remove the other layers, which no Worker started from here reads
        for (const entry of await files.readDirectory(directory)) {
            const path = join(directory, entry.name);
            if (path !== layer) {
                await files.remove(path, { recursive: true, force: true });
            }
        }

        return layer;
    }

    /** Write the manifest of the releases computers of a platform run to a directory. */
    async writeDistribution(directory: string, platform: DistributionPlatform): Promise<void> {
        // write the platform's manifest into the directory, created if new
        const { files } = this.store.environment;
        const distribution = this.distribution(platform);
        const text = JSON.stringify(DistributionManifest.parse(distribution));
        await files.mkdir(directory, { recursive: true });
        await files.writeTextFile(join(directory, DISTRIBUTION_MANIFEST), text);
    }

    /** Write the releases with what computers run as a layer, as local cloud machines read it. */
    async writeLayer(directory: string): Promise<void> {
        await this.store.writeLayer(
            [...this.images.values()],
            COMPUTER_RUNTIMES,
            join(directory, DISTRIBUTION_LAYER),
            "linked",
        );
    }

    /** List the stacks spaces people create are born with, following the stack package's tag. */
    stacks(tag: string): PlatformDefinition["stacks"] {
        return {
            home: this.following(EXPORTS.home, tag),
            space: this.following(EXPORTS.space, tag),
        };
    }

    /** Follow an export of the stack package at a tag, as a space people create does. */
    following(exported: keyof typeof stacks, tag: string): StackRelease {
        const stack = present(this.releases.get(STACK), `the release of ${STACK}`);

        return {
            packageId: PackageManifest.package(stack.manifest).id,
            selection: { kind: "tag", tag },
            export: exported,
            settings: {},
            routes: {},
        };
    }

    /** Select an export of the stack package's built release with what a home places in it. */
    released(
        exported: keyof typeof stacks,
        placed: Pick<StackRelease, "settings" | "routes">,
    ): StackRelease {
        // select the stack package's built release by its version
        const stack = present(this.releases.get(STACK), `the release of ${STACK}`);
        const { id, version } = PackageManifest.package(stack.manifest);

        return {
            packageId: id,
            selection: { kind: "release", version },
            export: exported,
            settings: { ...placed.settings },
            routes: { ...placed.routes },
        };
    }
}

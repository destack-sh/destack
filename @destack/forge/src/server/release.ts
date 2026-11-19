import { type BuildStore } from "@destack/build/store";
import { BuildImage, PackageManifest } from "@destack/package/manifest";
import { type DependencyRelease, type PackageId, PackageName } from "@destack/package";
import { type MethodContextOf, type NextOf, type ResultOf } from "@destack/object";
import { Vocabulary } from "@destack/resource";
import { type Digest, type Identifier, present, Version, schema } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { Snapshot } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import { SettingValue } from "@destack/setting/object";
import { buildRetention } from "@destack/stack/setting";
import { NpmTarball } from "@destack/registry/npm";
import { Forge } from "./forge.ts";
import { ForgeError } from "../error/index.ts";
import {
    LATEST_TAG,
    type Candidate,
    packageObject,
    type PackageVisibility,
    release,
    tag,
    TagName,
} from "@destack/registry/object";

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

/** A package a publish releases, checked before its transaction. */
type Publication = MethodContextOf<typeof packageObject, "publish">["prepared"][number];

/** What releases check builds against: the store of pushed builds. */
export interface ReleaseOptions {
    /** The store of pushed builds. */
    readonly store: BuildStore;
}

/** The release methods' handlers: preparing, publishing, resolving, deprecating and yanking. */
export const ReleaseMethod = {
    /** Check and copy up each of the account's packages in a pushed image, required ones first. */
    async prepare(
        context: MethodContextOf<
            typeof packageObject,
            "publish",
            readonly Publication[] | undefined
        >,
        options: ReleaseOptions,
    ): Promise<readonly Publication[]> {
        // refuse a tag a range reads, and lease the pushed image through the publish
        const { store } = options;
        TagName.require(context.input.tag ?? LATEST_TAG);
        const { image } = context.input;
        const request = present(context.requestId, "the release's request");
        const until = new Date(RequestId.expiry(request, context.now));
        await store.lease(image, until);

        // require a pushed image
        if (!(await store.contains(image))) {
            throw ForgeError.of("BUILD_NOT_FOUND", `image '${image}' not found`);
        }
        const read = await store.image(image);

        // take the packages of the root's handle within the image, required ones first
        const root = PackageManifest.package(await store.get(read.root));
        const { handle } = PackageName.split(root.name);
        const owned: { readonly digest: Digest; readonly manifest: PackageManifest }[] = [];
        for (const digest of BuildImage.layers(read).flat()) {
            const manifest = await store.get(digest);
            if (
                PackageManifest.isPackage(manifest) &&
                PackageName.split(manifest.package.name).handle === handle
            ) {
                owned.push({ digest, manifest });
            }
        }

        // check each in its own image, skipping a required one already released, as pnpm's recursive publish does
        const publishing = new Set<string>();
        const candidates: Publication[] = [];
        for (const { digest, manifest } of owned) {
            // leave a required package's version released before alone, tags included
            const required = PackageManifest.package(manifest);
            const [taken] = await context.read(release).findMany({
                where: { parentId: required.id, version: required.version },
                limit: 1,
            });
            if (digest !== read.root && taken !== undefined) {
                continue;
            }

            // check the root and each unreleased required one, counting it as released for those after it
            const at =
                digest === read.root ? image : await store.putImage(BuildImage.at(read, digest));
            await store.lease(at, until);
            const candidate = await ReleaseMethod.check(context, store, at, publishing);
            publishing.add(`${candidate.package.id}@${candidate.package.version}`);
            candidates.push(candidate);
        }

        return candidates;
    },

    /** Check and copy up one package's image before the publish's transaction. */
    async check(
        context: MethodContextOf<
            typeof packageObject,
            "publish",
            readonly Publication[] | undefined
        >,
        store: BuildStore,
        image: Digest,
        publishing: ReadonlySet<string>,
    ): Promise<Publication> {
        // read the image's root build
        const { accountId, commit } = context.input;
        const read = await store.image(image);
        const contents = await store.contents(read.root);

        // require the root build's graph, which every release carries for its readers
        if (!(await store.isIndexed(read.root))) {
            throw ForgeError.of(
                "BUILD_INVALID",
                `image '${image}' is invalid: its root build carries no graph`,
            );
        }

        // refuse a release of uncommitted changes that is no prerelease
        if (commit === undefined && !Version.isPrerelease(contents.manifest.package.version)) {
            throw ForgeError.of(
                "BUILD_INVALID",
                `image '${image}' is invalid: a release of uncommitted changes must be a prerelease`,
            );
        }

        // require the package's name to be the publishing account's, and not another package's
        const released = PackageManifest.package(contents.manifest);
        const { id, name, version } = released;
        const home = schema.identifier("space").parse(context.scope);
        const named = await Forge.find(name, context, home);
        if (named.accountId !== accountId) {
            throw ForgeError.of(
                "BUILD_INVALID",
                `image '${image}' is invalid: its root is a build of '${name}', not of the publishing account`,
            );
        } else if (named.package !== undefined && named.package.id !== id) {
            throw ForgeError.of("NPM_NAME_EXISTS", "npm name already exists");
        }

        // refuse a builtin outside the platform's organization
        if (contents.manifest.role === "builtin") {
            await Forge.requireBuiltin(name, context, home);
        }

        // refuse a locked package hosted here, the versions this publish releases counting as released
        const entries = await store.entriesOf(read);
        await ReleaseMethod.requirePublished(context, id, entries, publishing);

        // answer the release of this version again when another image of the same build published it
        const [taken] = await context
            .read(release)
            .findMany({ where: { parentId: id, version }, limit: 1 });
        const isRepeated =
            taken !== undefined &&
            taken.image !== image &&
            (await store.image(taken.image)).root === read.root;

        // copy up the image and every build it lists that the host's distribution ships
        const builds = Object.values(entries).map((entry) => ({
            kind: "manifest" as const,
            digest: entry.manifest,
        }));
        await store.copyUp([{ kind: "image", digest: image }, ...builds]);

        // pack the tarball npm clients download once, and store it for the release to refer to
        const { bytes, listed } = await NpmTarball.pack(store, image, context.environment);
        const tarball = await store.blobs.write(
            (async function* () {
                yield bytes;
            })(),
        );

        // read the visibility the package file asks through npm's access
        const authored = PackageAccess.parse(
            JSON.parse(new TextDecoder().decode(await contents.load(PACKAGE_FILE))),
        );
        const access = authored.publishConfig?.access;

        return {
            package: { id, name, version },
            ...(access === undefined ? {} : { visibility: NPM_ACCESS[access] }),
            release: {
                image: isRepeated ? taken.image : image,
                ...(commit === undefined ? {} : { commit }),
                version,
                role: contents.manifest.role,
                declarations: await contents.declarations(),
                npm: listed,
                tarball,
            },
        };
    },

    /** Insert a release of an unused version, or answer this image's, and point its tag at it. */
    async publish(
        context: MethodContextOf<typeof release, "create">,
        next: NextOf<typeof release, "create">,
    ): Promise<ResultOf<typeof release, "create">> {
        // answer the release this image published at its version
        const { parentId, tag: pointed, ...packed } = context.input;
        const published = await ReleaseMethod.published(context, packed);
        const answered = published ?? (await ReleaseMethod.insert(context, next, packed));

        // point its tag at it
        await context.mutate(context.caller, [
            packageObject.call("point", {
                id: parentId,
                name: pointed ?? LATEST_TAG,
                version: packed.version,
            }),
        ]);

        return answered;
    },

    /** Insert a release of a version after the newest, advancing the vocabulary. */
    async insert(
        context: MethodContextOf<typeof release, "create">,
        next: NextOf<typeof release, "create">,
        packed: Candidate,
    ): Promise<ResultOf<typeof release, "create">> {
        // require the next version before planning and advancing the vocabulary
        await ReleaseMethod.requireNextVersion(context, packed);
        await ReleaseMethod.advance(context, packed);

        // insert the release
        return await next(context);
    },

    /** Read the release of the image's version, refusing one of another image or a yanked one. */
    async published(
        context: MethodContextOf<typeof release, "create">,
        packed: Candidate,
    ): Promise<ResultOf<typeof release, "create"> | undefined> {
        // read the release of the version
        const [taken] = await context.read(release).findMany({
            where: { parentId: context.input.parentId, version: packed.version },
            limit: 1,
        });

        // refuse another image's release
        if (taken !== undefined && taken.image !== packed.image) {
            throw ForgeError.of(
                "RELEASE_EXISTS",
                `release '${packed.version}' of '${context.input.parentId}' already exists: another image published it`,
            );
        } else if (taken !== undefined && taken.yankedAt !== null) {
            throw ForgeError.of(
                "RELEASE_EXISTS",
                `release '${packed.version}' of '${context.input.parentId}' already exists: it is yanked`,
            );
        }

        return taken;
    },

    /** Refuse a hosted locked package whose locked version is unreleased or yanked, like Cargo. */
    async requirePublished(
        context: MethodContextOf<
            typeof packageObject,
            "publish",
            readonly Publication[] | undefined
        >,
        root: PackageId,
        entries: Readonly<Record<string, DependencyRelease>>,
        publishing: ReadonlySet<string>,
    ): Promise<void> {
        // read the hosted dependencies and their releases in two reads
        const dependencies = Object.values(entries).flatMap((entry) =>
            entry.kind === "destack" && entry.package.id !== root ? [entry.package] : [],
        );
        const hosted = await context
            .read(packageObject)
            .findMany({ where: { id: { in: dependencies.map((each) => each.id) } } });
        const released = await context
            .read(release)
            .findMany({ where: { parentId: { in: hosted.map((each) => each.id) } } });
        const releases = new Map(
            released.map((each) => [`${each.parentId}@${each.version}`, each]),
        );

        // refuse a hosted dependency whose locked version is yanked or not released, by this publish or before
        const ids = new Set(hosted.map((each) => each.id));
        for (const dependency of dependencies.filter((each) => ids.has(each.id))) {
            const key = `${dependency.id}@${dependency.version}`;
            const found = releases.get(key);
            const locked = `${dependency.name}@${dependency.version}`;

            // refuse a version yanked, which new dependents skip
            if (found !== undefined && found.yankedAt !== null) {
                throw ForgeError.of(
                    "DEPENDENCY_BLOCKED",
                    `cannot release with dependency '${locked}': it is yanked`,
                );
            }
            // refuse a version neither released nor in this publish
            else if (found === undefined && !publishing.has(key)) {
                throw ForgeError.of(
                    "DEPENDENCY_BLOCKED",
                    `cannot release with dependency '${locked}': it is not published`,
                );
            }
        }
    },

    /** Delete a prerelease, refusing a stable release, which stays immutable and only yanks. */
    async delete(
        call: MethodContextOf<typeof release, "delete">,
        next: NextOf<typeof release, "delete">,
    ): Promise<ResultOf<typeof release, "delete">> {
        if (!Version.isPrerelease(call.target.version)) {
            throw ForgeError.of(
                "RELEASE_INVALID",
                `release '${call.target.version}' is invalid: a stable release cannot be deleted, only yanked`,
            );
        }

        return await next(call);
    },

    /** Delete a package's prereleases past the build retention, keeping the newest and tagged. */
    async retain(store: ObjectStore, packageId: Identifier<"package">): Promise<void> {
        // read how many prereleases of each package the space keeps
        const { database, scope } = store;
        const snapshot = Snapshot.live(database);
        const kept = await SettingValue.resolve(snapshot, buildRetention, { scope });
        const space = schema.identifier("space").parse(scope);
        const releases = await snapshot
            .read(release.table)
            .findMany({ where: { scope: space, parentId: packageId } });
        const tagged = new Set(
            (
                await snapshot
                    .read(tag.table)
                    .findMany({ where: { scope: space, parentId: packageId } })
            ).map((each) => `${each.parentId} ${each.version}`),
        );

        // drop each package's untagged prereleases past the newest it keeps
        const prereleases = releases
            .filter((each) => Version.isPrerelease(each.version))
            .toSorted((left, right) => right.createdAt - left.createdAt);
        const dropped = [...Map.groupBy(prereleases, (each) => each.parentId).values()].flatMap(
            (newestFirst) =>
                newestFirst
                    .slice(kept)
                    .filter((each) => !tagged.has(`${each.parentId} ${each.version}`)),
        );

        // delete them
        if (dropped.length > 0) {
            await store.mutate(
                store.as(release),
                {
                    id: RequestId.create(store.environment),
                    calls: dropped.map((each) => release.call("delete", release.target(each))),
                },
                { component: "retention" },
            );
        }
    },

    /** Find a package's published release of a version, or of the version a tag points at. */
    async resolve(
        context: MethodContextOf<typeof release, "resolve">,
    ): Promise<ResultOf<typeof release, "resolve">> {
        // read the version a distribution tag points at, else the version given
        const { parentId, release: named } = context.input;
        const [tagged] = TagName.safeParse(named).success
            ? await context.read(tag).findMany({ where: { parentId, name: named } })
            : [];
        const version = tagged?.version ?? named;

        // find the published release of the version
        const [published] = await context
            .read(release)
            .findMany({ where: { parentId, version, yankedAt: { isNull: true } } });
        if (published === undefined) {
            throw ForgeError.of(
                "RELEASE_NOT_FOUND",
                `release '${named}' of '${parentId}' not found`,
            );
        }

        return published;
    },

    /** Deprecate a release with the warning installers show, or withdraw its deprecation. */
    async deprecate(
        context: MethodContextOf<typeof release, "deprecate">,
    ): Promise<ResultOf<typeof release, "deprecate">> {
        return context.update({ deprecation: context.input.message });
    },

    /** Yank a release from new installs, moving its tags off it, or restore it. */
    async yank(
        context: MethodContextOf<typeof release, "yank">,
    ): Promise<ResultOf<typeof release, "yank">> {
        // restore a yanked release, its tags left where they moved
        const yanked = context.target;
        if (!context.input.isYanked) {
            return context.update({ yankedAt: null });
        }

        // delete the tags pointing at it
        const pointing = await context
            .read(tag)
            .findMany({ where: { parentId: yanked.parentId, version: yanked.version } });
        await context.mutate(
            context.caller,
            pointing.map((each) => tag.call("delete", { id: each.id })),
        );

        // point latest at the highest version not yanked, the newest since releases publish in version order
        const [newest] = await context.read(release).findMany({
            where: {
                parentId: yanked.parentId,
                version: { ne: yanked.version },
                yankedAt: { isNull: true },
            },
            orderBy: { createdAt: "desc", id: "desc" },
            limit: 1,
        });
        const highest = newest?.version;
        if (pointing.some((each) => each.name === LATEST_TAG) && highest !== undefined) {
            await context.mutate(context.caller, [
                packageObject.call("point", {
                    id: yanked.parentId,
                    name: LATEST_TAG,
                    version: highest,
                }),
            ]);
        }

        return context.update({ yankedAt: yanked.yankedAt ?? context.now });
    },

    /** Refuse a version older than the package's newest release. */
    async requireNextVersion(
        context: MethodContextOf<typeof release, "create">,
        packed: Candidate,
    ): Promise<void> {
        // require releases in version order, the newest release being the highest version, ties in creation order
        const [newest] = await context.read(release).findMany({
            where: { parentId: context.input.parentId },
            orderBy: { createdAt: "desc", id: "desc" },
            limit: 1,
        });
        const latest = newest?.version;
        if (latest !== undefined && Version.compare(packed.version, latest) < 0) {
            throw ForgeError.of(
                "VERSION_BLOCKED",
                `cannot publish release '${packed.version}': it precedes '${latest}', and releases publish in version order`,
            );
        }
    },

    /** Advance the package's vocabulary and role to a release. */
    async advance(
        context: MethodContextOf<typeof release, "create">,
        packed: Candidate,
    ): Promise<void> {
        // read the package's vocabulary
        const parentId = context.input.parentId;
        const owner = await context.read(packageObject).findFirst({ where: { id: parentId } });
        if (owner === undefined) {
            throw new TypeError(`release package is missing: '${parentId}'`);
        }

        // plan the declarations, refusing a plan that breaks the terms
        Vocabulary.plan(owner.vocabulary, packed.declarations);

        // advance the vocabulary to the release
        await context.mutate(context.caller, [
            packageObject.call("advance", {
                id: parentId,
                vocabulary: Vocabulary.advance(
                    owner.vocabulary,
                    packed.declarations,
                    packed.version,
                ),
                role: packed.role,
            }),
        ]);
    },
};

/** Implement releases. */
export function implementRelease(): typeof release {
    return release.implement({
        methods: {
            create: (context, next) => ReleaseMethod.publish(context, next),
            resolve: (call) => ReleaseMethod.resolve(call),
            deprecate: (call) => ReleaseMethod.deprecate(call),
            yank: (call) => ReleaseMethod.yank(call),
            delete: (call, next) => ReleaseMethod.delete(call, next),
        },
    });
}

import { type BuildStore } from "@destack/build/store";
import { PackageManifest } from "@destack/package/manifest";
import { type DependencyRelease, type PackageId, PackageName } from "@destack/package";
import { type MethodContextOf, type NextOf, type ResultOf } from "@destack/object";
import { Vocabulary } from "@destack/resource";
import { present, Version, schema } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { Projection, Snapshot } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import { SettingValue } from "@destack/setting/object";
import { buildRetention } from "@destack/stack/setting";
import { Forge } from "./forge.ts";
import { ForgeError } from "../error/index.ts";
import {
    LATEST_TAG,
    type Candidate,
    packageObject,
    release,
    tag,
    TagName,
} from "@destack/registry/object";

/** What releases check builds against: the store of pushed builds. */
export interface ReleaseOptions {
    /** The store of pushed builds. */
    readonly store: BuildStore;
}

/** The release methods' handlers: preparing, publishing, resolving, deprecating and yanking. */
export const ReleaseMethod = {
    /** Check and copy up a pushed lock before the publish's transaction. */
    async prepare(
        context: MethodContextOf<typeof packageObject, "publish", Candidate | undefined>,
        options: ReleaseOptions,
    ): Promise<Candidate> {
        // refuse a tag a range reads
        const store = options.store;
        TagName.require(context.input.tag ?? LATEST_TAG);
        const { packageId: parentId, accountId, name: local, lock, commit } = context.input;
        const request = present(context.requestId, "the release's request");
        const until = new Date(RequestId.expiry(request, context.now));
        await store.lease(lock, until);

        // require a pushed lock
        if (!(await store.contains(lock))) {
            throw ForgeError.of("BUILD_NOT_FOUND", `lock '${lock}' not found`);
        }
        const read = await store.lock(lock);
        const contents = await store.contents(read.root);

        // refuse a release of uncommitted changes that is no prerelease
        if (commit === undefined && !Version.isPrerelease(contents.manifest.package.version)) {
            throw ForgeError.of(
                "BUILD_INVALID",
                `lock '${lock}' is invalid: a release of uncommitted changes must be a prerelease`,
            );
        }

        // require the build to be the package's, under the package's global name
        const { id, name, version } = PackageManifest.package(contents.manifest);
        if (id !== parentId) {
            throw ForgeError.of("BUILD_INVALID", "build is invalid: it belongs to another package");
        }
        const home = schema.identifier("space").parse(context.scope);
        const named = await Forge.find(name, context, home);
        if (named.accountId !== accountId || PackageName.split(name).local !== local) {
            throw ForgeError.of(
                "BUILD_INVALID",
                `lock '${lock}' is invalid: its root is a build of '${name}', not of the published package`,
            );
        } else if (named.package !== undefined && named.package.id !== parentId) {
            throw ForgeError.of("NPM_NAME_EXISTS", "npm name already exists");
        }

        // refuse a builtin outside the platform's organization
        if (contents.manifest.role === "builtin") {
            await Forge.requireBuiltin(name, context, home);
        }

        // refuse a locked package hosted here
        const entries = await store.entriesOf(read);
        await ReleaseMethod.requirePublished(context, id, entries);

        // answer the release of this version again when another lock of the same build published it
        const [taken] = await context
            .read(release)
            .findMany({ where: { parentId, version }, limit: 1 });
        const isRepeated =
            taken !== undefined &&
            taken.lock !== lock &&
            (await store.lock(taken.lock)).root === read.root;

        // copy up the lock and every build it lists that the host's distribution ships
        const builds = Object.values(entries).map((entry) => ({
            kind: "manifest" as const,
            digest: entry.manifest,
        }));
        await store.copyUp([{ kind: "lock", digest: lock }, ...builds]);

        // read the root's declarations
        return {
            lock: isRepeated ? taken.lock : lock,
            ...(commit === undefined ? {} : { commit }),
            version,
            declarations: await contents.declarations(),
        };
    },

    /** Insert a release of an unused version, or answer this lock's, and point its tag at it. */
    async publish(
        context: MethodContextOf<typeof release, "create">,
        next: NextOf<typeof release, "create">,
    ): Promise<ResultOf<typeof release, "create">> {
        // answer the release this lock published at its version
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

    /** Read the release of the lock's version, refusing one of another lock or a yanked one. */
    async published(
        context: MethodContextOf<typeof release, "create">,
        packed: Candidate,
    ): Promise<ResultOf<typeof release, "create"> | undefined> {
        // read the release of the version
        const [taken] = await context.read(release).findMany({
            where: { parentId: context.input.parentId, version: packed.version },
            limit: 1,
        });

        // refuse another lock's release
        if (taken !== undefined && taken.lock !== packed.lock) {
            throw ForgeError.of(
                "RELEASE_EXISTS",
                `release '${packed.version}' of '${context.input.parentId}' already exists: another lock published it`,
            );
        } else if (taken !== undefined && taken.yankedAt !== null) {
            throw ForgeError.of(
                "RELEASE_EXISTS",
                `release '${packed.version}' of '${context.input.parentId}' already exists: it is yanked`,
            );
        }

        return taken;
    },

    /** Refuse a locked package this forge hosts whose locked version is not released. */
    async requirePublished(
        context: MethodContextOf<typeof packageObject, "publish", Candidate | undefined>,
        root: PackageId,
        entries: Readonly<Record<string, DependencyRelease>>,
    ): Promise<void> {
        // read the hosted dependencies and their releases in two reads
        const dependencies = Object.values(entries).flatMap((entry) =>
            entry.kind === "destack" && entry.package.id !== root ? [entry.package] : [],
        );
        const hosted = await context
            .read(packageObject)
            .findMany({ where: { id: { in: dependencies.map((each) => each.id) } } });
        const released = await context.read(release).findMany({
            where: { parentId: { in: hosted.map((each) => each.id) }, yankedAt: { isNull: true } },
        });
        const versions = new Set(released.map((each) => `${each.parentId}@${each.version}`));

        // refuse a hosted dependency whose locked version is not released
        const ids = new Set(hosted.map((each) => each.id));
        const unreleased = dependencies.find(
            (each) => ids.has(each.id) && !versions.has(`${each.id}@${each.version}`),
        );
        if (unreleased !== undefined) {
            throw ForgeError.of(
                "DEPENDENCY_BLOCKED",
                `cannot release with dependency '${unreleased.name}@${unreleased.version}': it is not published`,
            );
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

    /** Delete each package's prereleases past its build retention, keeping tagged ones. */
    retention(store: ObjectStore): Projection {
        return new Projection(store.database, {
            name: "release-retention",
            tables: [release.table, tag.table],
            keys: async () => [store.scope],
            all: async () => [store.scope],
            project: async () => ReleaseMethod.retain(store),
        });
    },

    /** Delete prereleases past the build retention, keeping the newest and the tagged ones. */
    async retain(store: ObjectStore): Promise<void> {
        // read how many prereleases of each package the space keeps
        const { database, scope } = store;
        const snapshot = Snapshot.live(database);
        const kept = await SettingValue.resolve(snapshot, buildRetention, { scope });
        const space = schema.identifier("space").parse(scope);
        const releases = await snapshot.read(release.table).findMany({ where: { scope: space } });
        const tagged = new Set(
            (await snapshot.read(tag.table).findMany({ where: { scope: space } })).map(
                (each) => `${each.parentId} ${each.version}`,
            ),
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

    /** Plan the package's vocabulary through a release's declarations and advance it. */
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

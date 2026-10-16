import { type PackageStore } from "@destack/build/store";
import { type MethodContextOf, type NextOf, type ResultOf } from "@destack/object";
import { Upgrade, Vocabulary } from "@destack/resource";
import { type Identifier, present, Version } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { CallError } from "@destack/runtime/error";
import { Scope } from "@destack/sync";
import { LATEST_TAG, packageObject, release, tag, TagName } from "../object/index.ts";
import type { Pack } from "../pack/index.ts";
import { PackageArchive } from "../pack/index.ts";
import { claimFollow, Forge } from "./forge.ts";

/** What releases pack with: the store of pushed builds and the forge's space. */
export interface ReleaseOptions {
    /** The store of pushed builds and npm archives. */
    readonly store: PackageStore;
    /** The forge's space, which stores every package of its residency. */
    readonly home: Identifier<"space">;
}

/** The release methods' handlers: packing, publishing, resolving, deprecating and yanking. */
export const ReleaseMethod = {
    /** Pack a pushed build under the package's global name, before the transaction. */
    async pack(
        context: MethodContextOf<typeof release, "create", Pack | undefined>,
        options: ReleaseOptions,
    ): Promise<Pack> {
        // refuse a tag a range reads, and lease the build against sweeps while its request may still retry
        const store = options.store;
        TagName.require(context.input.tag ?? LATEST_TAG);
        const { parentId, manifest } = context.input;
        const request = present(context.requestId, "the release's request");
        const until = new Date(RequestId.expiry(request, context.now));
        await store.lease(manifest, until);

        // require a pushed build, of a commit unless it is a prerelease
        if (!(await store.contains(manifest))) {
            throw new CallError("NOT_FOUND", {
                message: `build ${manifest} was not pushed to the forge`,
            });
        }
        const contents = await store.contents(manifest);
        const { commit, package: built } = contents.manifest;
        if (commit === undefined && !Version.isPrerelease(built.version)) {
            throw new CallError("CONFLICT", {
                message: `build ${manifest} compiled a working tree with uncommitted changes`,
            });
        }

        // pack the build, refusing an archive of another manifest
        const archive = await PackageArchive.pack(contents, commit);
        if (archive.manifest !== manifest) {
            throw new CallError("BAD_REQUEST", {
                message: "archived manifest differs from the build",
            });
        }

        // require the build to be the package's, under the package's global name
        if (archive.packageId !== parentId) {
            throw new CallError("CONFLICT", { message: "the build is another package's" });
        }
        const named = await Forge.named(
            archive.metadata.name,
            options.home,
            (claim) =>
                context.acquire(claimFollow, { key: { scope: Scope.universe.id, ...claim } }),
            context,
        );
        if (named.id !== parentId) {
            throw new CallError("CONFLICT", {
                message: "the npm name belongs to another package",
            });
        }

        // lease and store the archive under its digest before any release refers to it
        await store.lease(archive.file.digest, until);
        await store.putFile(archive.file, async () => archive.open());

        // read the build's declarations and the upgrade it planned
        const upgrade = contents.manifest.upgrade;

        return {
            manifest,
            ...(commit === undefined ? {} : { commit }),
            version: archive.metadata.version,
            distribution: archive.distribution,
            metadata: archive.metadata,
            dependencies: archive.dependencies,
            ...(upgrade === undefined
                ? {}
                : { upgrade: await contents.read(upgrade.file, Upgrade) }),
            declarations: await contents.declarations(),
        };
    },

    /** Insert a release of an unused version, record its dependencies, and point its tag at it. */
    async publish(
        context: MethodContextOf<typeof release, "create">,
        next: NextOf<typeof release, "create">,
    ): Promise<ResultOf<typeof release, "create">> {
        // require the next version before planning and advancing the vocabulary
        const packed = context.prepared;
        await ReleaseMethod.requireNextVersion(context, packed);
        await ReleaseMethod.advance(context, packed);

        // insert the release
        const { manifest, commit, version, distribution, metadata, upgrade, declarations } = packed;
        const created = await next(
            context.with({
                input: {
                    ...context.input,
                    manifest,
                    ...(commit === undefined ? {} : { commit }),
                    version,
                    distribution,
                    metadata,
                    declarations,
                    ...(upgrade === undefined ? {} : { upgrade }),
                },
            }),
        );

        // point its tag at it, latest by default
        await context.mutate(context.caller, [
            packageObject.call("point", {
                id: context.input.parentId,
                name: context.input.tag ?? LATEST_TAG,
                version: packed.version,
            }),
        ]);

        return created;
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
            throw new CallError("NOT_FOUND", { message: `${parentId} lacks release ${named}` });
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
            orderBy: { createdAt: "desc" },
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

    /** Refuse a version the package took before, and an older one. */
    async requireNextVersion(
        context: MethodContextOf<typeof release, "create">,
        packed: Pack,
    ): Promise<void> {
        // refuse a version the package took before, yanked or not
        const [taken] = await context.read(release).findMany({
            where: { parentId: context.input.parentId, version: packed.version },
            limit: 1,
        });
        if (taken !== undefined) {
            throw new CallError("CONFLICT", {
                message: "package version was already published",
            });
        }

        // require releases in version order, the newest release being the highest version
        const [newest] = await context.read(release).findMany({
            where: { parentId: context.input.parentId },
            orderBy: { createdAt: "desc" },
            limit: 1,
        });
        const latest = newest?.version;
        if (latest !== undefined && Version.compare(packed.version, latest) < 0) {
            throw new CallError("CONFLICT", {
                message: `publish releases in version order: ${packed.version} precedes ${latest}`,
            });
        }
        // refuse an upgrade on the first release
        else if (latest === undefined && packed.upgrade !== undefined) {
            throw new CallError("CONFLICT", { message: "the first release plans no upgrade" });
        }
        // require an upgrade planned from the latest release
        else if (latest !== undefined && packed.upgrade?.from !== latest) {
            throw new CallError("CONFLICT", {
                message: `build the release against ${latest}, the latest release`,
            });
        }
    },

    /** Plan the package's vocabulary through a release's declarations and advance it. */
    async advance(context: MethodContextOf<typeof release, "create">, packed: Pack): Promise<void> {
        // read the package's vocabulary
        const parentId = context.input.parentId;
        const owner = await context.read(packageObject).findFirst({ where: { id: parentId } });
        if (owner === undefined) {
            throw new TypeError(`release package is missing: ${parentId}`);
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
export function implementRelease(options: ReleaseOptions): typeof release {
    return release.implement({
        methods: {
            create: {
                prepare: (context) => ReleaseMethod.pack(context, options),
                handle: (context, next) => ReleaseMethod.publish(context, next),
            },
            resolve: (call) => ReleaseMethod.resolve(call),
            deprecate: (call) => ReleaseMethod.deprecate(call),
            yank: (call) => ReleaseMethod.yank(call),
        },
    });
}

import { type BuildStore } from "@destack/build/store";
import { PackageManifest } from "@destack/package/manifest";
import { type MethodContextOf, type NextOf, type ResultOf } from "@destack/object";
import { Vocabulary } from "@destack/resource";
import { present, Version, schema } from "@destack/schema";
import { RequestId } from "@destack/sync";
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
import { unit } from "../object/index.ts";

/** What releases check builds against: the store of pushed builds. */
export interface ReleaseOptions {
    /** The store of pushed builds. */
    readonly store: BuildStore;
}

/** The release methods' handlers: checking, publishing, resolving, deprecating and yanking. */
export const ReleaseMethod = {
    /** Check a pushed build of the package under its global name, before the transaction. */
    async check(
        context: MethodContextOf<typeof release, "create", Candidate | undefined>,
        options: ReleaseOptions,
    ): Promise<Candidate> {
        // refuse a tag a range reads, and lease the build while its request may still retry
        const store = options.store;
        TagName.require(context.input.tag ?? LATEST_TAG);
        const { parentId, manifest } = context.input;
        const request = present(context.requestId, "the release's request");
        const until = new Date(RequestId.expiry(request, context.now));
        await store.lease(manifest, until);

        // require a pushed build, of a commit unless it is a prerelease
        if (!(await store.contains(manifest))) {
            throw ForgeError.of(
                "BUILD_NOT_PUSHED",
                `build ${manifest} was not pushed to the forge`,
            );
        }
        const contents = await store.contents(manifest);
        const { commit, package: built } = contents.manifest;
        if (commit === undefined && !Version.isPrerelease(built.version)) {
            throw ForgeError.of(
                "UNCOMMITTED_CHANGES",
                `build ${manifest} compiled a working tree with uncommitted changes`,
            );
        }

        // require the build to be the package's, under the package's global name
        const { id, name, version } = PackageManifest.package(contents.manifest);
        if (id !== parentId) {
            throw ForgeError.of("FOREIGN_BUILD", "the build belongs to another package");
        }
        const named = await Forge.package(
            name,
            context,
            schema.identifier("space").parse(context.scope),
        );
        if (named.id !== parentId) {
            throw ForgeError.of("NPM_NAME_TAKEN", "the npm name belongs to another package");
        }

        // lease the npm units it depends on, which its readers read with it
        const units = await store.units(manifest, parentId);
        for (const each of units) {
            await store.lease(each, until);
        }

        // read the build's declarations
        return {
            manifest,
            ...(commit === undefined ? {} : { commit }),
            version,
            units,
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
        const { manifest, commit, version } = packed;
        const created = await next(
            context.with({
                input: {
                    ...context.input,
                    manifest,
                    ...(commit === undefined ? {} : { commit }),
                    version,
                },
            }),
        );

        // record its npm units, and point its tag at it, latest by default
        await context.mutate(context.caller, [
            ...packed.units.map((digest) =>
                unit.call("create", { parentId: created.id, manifest: digest }),
            ),
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
            throw ForgeError.of("RELEASE_NOT_FOUND", `${parentId} has no release ${named}`);
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

    /** Refuse a version the package took before, and an older one. */
    async requireNextVersion(
        context: MethodContextOf<typeof release, "create">,
        packed: Candidate,
    ): Promise<void> {
        // refuse a version the package took before, yanked or not
        const [taken] = await context.read(release).findMany({
            where: { parentId: context.input.parentId, version: packed.version },
            limit: 1,
        });
        if (taken !== undefined) {
            throw ForgeError.of("VERSION_PUBLISHED", "package version was already published");
        }

        // require releases in version order, the newest release being the highest version, ties in creation order
        const [newest] = await context.read(release).findMany({
            where: { parentId: context.input.parentId },
            orderBy: { createdAt: "desc", id: "desc" },
            limit: 1,
        });
        const latest = newest?.version;
        if (latest !== undefined && Version.compare(packed.version, latest) < 0) {
            throw ForgeError.of(
                "VERSION_ORDER",
                `publish releases in version order: ${packed.version} precedes ${latest}`,
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
                prepare: (context) => ReleaseMethod.check(context, options),
                handle: (context, next) => ReleaseMethod.publish(context, next),
            },
            resolve: (call) => ReleaseMethod.resolve(call),
            deprecate: (call) => ReleaseMethod.deprecate(call),
            yank: (call) => ReleaseMethod.yank(call),
        },
    });
}

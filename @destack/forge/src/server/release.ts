import { type MethodContextOf, type NextOf, type ResultOf } from "@destack/object";
import { Vocabulary } from "@destack/resource";
import { Version } from "@destack/schema";
import { ForgeError } from "../error/index.ts";
import { Forge } from "./forge.ts";
import {
    LATEST_TAG,
    type Candidate,
    packageObject,
    release,
    tag,
    TagName,
} from "@destack/registry/object";

/** Implement releases. */
export function implementRelease(): typeof release {
    return release.implement({
        methods: { create, resolve, deprecate, yank, delete: remove },
    });
}

/** Insert a release of an unused version, or answer this image's, and point its tag at it. */
async function create(
    context: MethodContextOf<typeof release, "create">,
    next: NextOf<typeof release, "create">,
): Promise<ResultOf<typeof release, "create">> {
    // answer the release this image published at its version
    const { parentId, tag: pointed, ...packed } = context.input;
    const published = await existing(context, packed);
    const answered = published ?? (await insert(context, next, packed));

    // point its tag at it
    await context.mutate(context.caller, [
        packageObject.call("point", {
            id: parentId,
            name: pointed ?? LATEST_TAG,
            version: packed.version,
        }),
    ]);

    return answered;
}
/** Insert a release of a version after the newest, advancing the vocabulary. */
async function insert(
    context: MethodContextOf<typeof release, "create">,
    next: NextOf<typeof release, "create">,
    packed: Candidate,
): Promise<ResultOf<typeof release, "create">> {
    // require the next version before planning and advancing the vocabulary
    await requireNextVersion(context, packed);
    await advance(context, packed);

    // insert the release
    return await next(context);
}
/** Read the release of the image's version, refusing one of another image or a yanked one. */
async function existing(
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
}
/** Delete a prerelease, refusing a stable release, which stays immutable and only yanks. */
async function remove(
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
}
/** Find a package's published release of a version, or of the version a tag points at. */
async function resolve(
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
        throw ForgeError.of("RELEASE_NOT_FOUND", `release '${named}' of '${parentId}' not found`);
    }

    return published;
}
/** Deprecate a release with the warning installers show, or withdraw its deprecation. */
async function deprecate(
    context: MethodContextOf<typeof release, "deprecate">,
): Promise<ResultOf<typeof release, "deprecate">> {
    return context.update({ deprecation: context.input.message });
}
/** Yank a release from new installs, moving its tags off it, or restore it. */
async function yank(
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
}
/** Refuse a version before the package's newest release. */
async function requireNextVersion(
    context: MethodContextOf<typeof release, "create">,
    packed: Candidate,
): Promise<void> {
    // order every version after the newest release, dev builds in any order among themselves
    const latest = await Forge.newest(context.input.parentId, context);
    if (latest !== undefined && Version.compare(packed.version, latest) < 0) {
        throw ForgeError.of(
            "VERSION_BLOCKED",
            `cannot publish release '${packed.version}': it precedes '${latest}', and releases publish in version order`,
        );
    }
}
/** Advance the package's vocabulary and role to a release. */
async function advance(
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
            vocabulary: Vocabulary.advance(owner.vocabulary, packed.declarations, packed.version),
            role: packed.role,
        }),
    ]);
}

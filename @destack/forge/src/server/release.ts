import { and, eq, isNull, ne, type DatabaseConnection, type Select } from "@destack/db";
import {
    Call,
    type CallOf,
    type NextOf,
    type PreparedCallOf,
    type ResultOf,
} from "@destack/object";
import { Vocabulary } from "@destack/resource";
import { Version } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { dependency, LATEST_TAG, packageObject, release, tag, type Pack } from "../object/index.ts";

/** How long after publication a release may be unpublished, in milliseconds: npm's 72 hours. */
const UNPUBLISH_WINDOW_MILLISECONDS = 72 * 60 * 60 * 1000;

/** A release's publication and unpublication on the server. */
export const Release = { tagName, publish, unpublish };

/** A release's row. */
type Row = Select<typeof release.table>;

/** Read the tag a release's creation names, latest when it names none. */
function tagName(call: CallOf<typeof release, "create">): string {
    return call.input.tag ?? LATEST_TAG;
}

/** Insert a release of an unused version, record its dependencies, and point its tag at it. */
async function publish(
    call: PreparedCallOf<typeof release, "create">,
    next: NextOf<typeof release, "create">,
): Promise<ResultOf<typeof release, "create">> {
    // require the next version before planning and advancing the vocabulary
    const pack = call.prepared;
    await requireNextVersion(call, pack);
    await advance(call, pack);

    // insert the release
    const { manifest, commit, version, distribution, metadata, upgrade } = pack;
    const created = await next(
        call.with({
            input: {
                ...call.input,
                manifest,
                commit,
                version,
                distribution,
                metadata,
                ...(upgrade === undefined ? {} : { upgrade }),
            },
        }),
    );

    // record each exact release it requires
    const releaseId = Call.resultId(created);
    if (releaseId === undefined) {
        throw new TypeError("release creation returned no id");
    }
    for (const required of pack.dependencies) {
        await call.invoke(dependency).create({ parentId: releaseId, ...required });
    }

    // point its tag at it, latest unless the call names another
    await call.invoke(tag).point({
        parentId: call.input.parentId,
        name: tagName(call),
        version: pack.version,
    });

    return created;
}

/** Unpublish a release as npm allows, dropping its tags, and keep it as a tombstone. */
async function unpublish(
    call: CallOf<typeof release, "unpublish">,
): Promise<ResultOf<typeof release, "unpublish">> {
    // refuse a release already unpublished
    const unpublished = call.target;
    if (unpublished.unpublishedAt !== null) {
        throw new ServiceError("CONFLICT", { message: "release is already unpublished" });
    }
    // refuse a release past the window
    else if (call.now - unpublished.createdAt >= UNPUBLISH_WINDOW_MILLISECONDS) {
        throw new ServiceError("CONFLICT", {
            message: "releases older than 72 hours stay published: deprecate them instead",
        });
    }

    // refuse while another package's published release requires it, else delete its tags
    await requireNoDependents(call.database, unpublished);
    const pointing = await untag(call, unpublished);

    // point latest at the highest version still published, when it pointed at this one
    const remaining = await call.database
        .select({ version: release.table.version })
        .from(release.table)
        .where(
            and(
                eq(release.table.parentId, unpublished.parentId),
                ne(release.table.version, unpublished.version),
                isNull(release.table.unpublishedAt),
            ),
        );
    const [highest] = remaining
        .map((entry) => entry.version)
        .toSorted((left, right) => Version.compare(right, left));
    if (pointing.includes(LATEST_TAG) && highest !== undefined) {
        await call.invoke(tag).point({
            parentId: unpublished.parentId,
            name: LATEST_TAG,
            version: highest,
        });
    }

    return call.update({ unpublishedAt: call.now });
}

/** Refuse a version the package took before, an older one, and one planned from another release than the latest. */
async function requireNextVersion(
    call: PreparedCallOf<typeof release, "create">,
    pack: Pack,
): Promise<void> {
    // refuse a version the package took before, published or unpublished
    const versions = (
        await call.database
            .select({ version: release.table.version })
            .from(release.table)
            .where(eq(release.table.parentId, call.input.parentId))
    ).map((row) => row.version);
    if (versions.includes(pack.version)) {
        throw new ServiceError("CONFLICT", {
            message: "package version was already published",
        });
    }

    // require releases in version order
    const latest = versions.toSorted((left, right) => Version.compare(left, right)).at(-1);
    if (latest !== undefined && Version.compare(pack.version, latest) < 0) {
        throw new ServiceError("CONFLICT", {
            message: `publish releases in version order: ${pack.version} precedes ${latest}`,
        });
    }
    // refuse an upgrade on the first release
    else if (latest === undefined && pack.upgrade !== undefined) {
        throw new ServiceError("CONFLICT", { message: "the first release plans no upgrade" });
    }
    // require an upgrade planned from the latest release
    else if (latest !== undefined && pack.upgrade?.from !== latest) {
        throw new ServiceError("CONFLICT", {
            message: `build the release against ${latest}, the latest release`,
        });
    }
}

/** Plan the package's vocabulary through a release's declarations and advance it to the release. */
async function advance(call: PreparedCallOf<typeof release, "create">, pack: Pack): Promise<void> {
    // read the package's vocabulary
    const parentId = call.input.parentId;
    const [owner] = await call.database
        .select({ vocabulary: packageObject.table.vocabulary })
        .from(packageObject.table)
        .where(eq(packageObject.table.id, parentId));
    if (owner === undefined) {
        throw new TypeError(`release package is missing: ${parentId}`);
    }

    // plan the declarations, refusing a plan that breaks the terms
    Vocabulary.plan(owner.vocabulary, pack.declarations);

    // advance the vocabulary to the release
    await call.invoke(packageObject).advance({
        id: parentId,
        vocabulary: Vocabulary.advance(owner.vocabulary, pack.declarations, pack.version),
    });
}

/** Refuse unpublishing a release another package's published release requires. */
async function requireNoDependents(database: DatabaseConnection, required: Row): Promise<void> {
    // NOTE #Incomplete: releases in other regions' databases may require it unseen
    const [dependent] = await database
        .select({ metadata: release.table.metadata })
        .from(dependency.table)
        .innerJoin(release.table, eq(release.table.id, dependency.table.parentId))
        .where(
            and(
                eq(dependency.table.name, required.metadata.name),
                eq(dependency.table.version, required.version),
                ne(release.table.parentId, required.parentId),
                isNull(release.table.unpublishedAt),
            ),
        )
        .limit(1);
    if (dependent !== undefined) {
        throw new ServiceError("CONFLICT", {
            message: `${dependent.metadata.name}@${dependent.metadata.version} depends on this release`,
        });
    }
}

/** Delete the tags pointing at a release, returning their names. */
async function untag(
    call: CallOf<typeof release, "unpublish">,
    unpublished: Row,
): Promise<string[]> {
    // find the tags pointing at it
    const pointing = await call.database
        .select({ id: tag.table.id, name: tag.table.name })
        .from(tag.table)
        .where(
            and(
                eq(tag.table.parentId, unpublished.parentId),
                eq(tag.table.version, unpublished.version),
            ),
        );

    // delete each
    for (const each of pointing) {
        await call.invoke(tag).delete({ id: each.id });
    }

    return pointing.map((each) => each.name);
}

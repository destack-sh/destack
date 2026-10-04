import { and, eq, isNull, type DatabaseConnection } from "@destack/db";
import type { CallOf, NextOf, ResultOf } from "@destack/object";
import { ServiceError } from "@destack/service/error";
import { validRange } from "semver";
import * as base from "../object/package.ts";
import { release, type Release } from "../object/package.ts";

/** Tags on the server, which point only at published releases. */
export const tag = base.tag.handle({ create, update, point });

/** Require a distribution tag name that no version range reads as a range. */
export function requireTagName(name: string): void {
    if (validRange(name) !== null) {
        throw new ServiceError("BAD_REQUEST", { message: `invalid distribution tag: ${name}` });
    }
}

/** Create a tag no range reads as one, pointing at a published release. */
async function create(
    call: CallOf<typeof base.tag, "create">,
    next: NextOf<typeof base.tag, "create">,
): Promise<ResultOf<typeof base.tag, "create">> {
    // require a tag name and a published target
    requireTagName(call.input.name);
    await requirePublished(call.database, call.input.parentId, call.input.version);

    return next();
}

/** Move a tag only to a published release. */
async function update(
    call: CallOf<typeof base.tag, "update">,
    next: NextOf<typeof base.tag, "update">,
): Promise<ResultOf<typeof base.tag, "update">> {
    // require a published target when the tag moves
    const version = call.input.version;
    if (version !== undefined) {
        await requirePublished(call.database, call.target.parentId, version);
    }

    return next();
}

/** Create a tag, or move the package's tag of the same name to the version. */
async function point(
    call: CallOf<typeof base.tag, "point">,
    next: NextOf<typeof base.tag, "point">,
): Promise<ResultOf<typeof base.tag, "point">> {
    // find the package's tag of the name
    const parentId = call.input.parentId;
    const name = call.input.name;
    const [existing] = await call.database
        .select({ id: base.tag.table.id })
        .from(base.tag.table)
        .where(and(eq(base.tag.table.parentId, parentId), eq(base.tag.table.name, name)));

    return existing === undefined
        ? next()
        : call.invoke(base.tag).update({ id: existing.id, version: call.input.version });
}

/** Require a version a tag may point at: a published release of the package. */
async function requirePublished(
    database: DatabaseConnection,
    packageId: Release["parentId"],
    version: string,
): Promise<void> {
    const [found] = await database
        .select({ id: release.table.id })
        .from(release.table)
        .where(
            and(
                eq(release.table.parentId, packageId),
                eq(release.table.version, version),
                isNull(release.table.unpublishedAt),
            ),
        );
    if (found === undefined) {
        throw new ServiceError("NOT_FOUND", {
            message: "distribution tag target is not published",
        });
    }
}

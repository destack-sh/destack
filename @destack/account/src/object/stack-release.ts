import { ReleaseVersion } from "@destack/object";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";

/** Where a space's stack comes from: a stack package's release, its export, settings and routes. */
export const StackRelease = schema.object({
    /** The stack package. */
    packageId: PackageId,
    /** The release the space runs: a version, a tag such as latest, or its host's. */
    version: ReleaseVersion,
    /** The export declaring the space. */
    export: schema.string().min(1),
    /** The setting values the stack places in the space, by `<package>/<name>`. */
    settings: schema.record(schema.string().min(1), schema.json()),
    /** The routes the stack places in the space beside its own, by key. */
    routes: schema.record(schema.string().min(1), schema.jsonObject()),
});
/** Where a space's stack comes from. */
export type StackRelease = schema.Infer<typeof StackRelease>;

/** The kind of a space, which decides its stack: a residency, person or account home, or other. */
export const SpaceKind = schema.enum(["residency", "person", "account", "space"]);
/** The kind of a space. */
export type SpaceKind = schema.Infer<typeof SpaceKind>;

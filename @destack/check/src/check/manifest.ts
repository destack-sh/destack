import { resolve } from "node:path";
import type { FileSystem } from "@destack/environment";
import { schema } from "@destack/schema";

/** The package.json fields checking reads. */
export const Manifest = schema.looseObject({
    /** The package name. */
    name: schema.string().exactOptional(),
    /** The runtime dependencies. */
    dependencies: schema.record(schema.string(), schema.string()).exactOptional(),
    /** The development dependencies. */
    devDependencies: schema.record(schema.string(), schema.string()).exactOptional(),
    /** The program a package runs as, or its programs by command name. */
    bin: schema
        .union([schema.string(), schema.record(schema.string(), schema.string())])
        .exactOptional(),
    /** The commands a package runs by name. */
    scripts: schema.record(schema.string(), schema.string()).exactOptional(),
    /** The modules a package exports by subpath, a string for each module file. */
    exports: schema.record(schema.string(), schema.unknown()).exactOptional(),
    /** The member directory patterns of a workspace root, a `*` segment matching any directory. */
    workspaces: schema.array(schema.string()).exactOptional(),
});
/** The package.json fields checking reads. */
export type Manifest = schema.Infer<typeof Manifest>;

/** Read a directory's package manifest, absent without one. */
export async function readManifest(
    directory: string,
    files: FileSystem,
): Promise<Manifest | undefined> {
    const path = resolve(directory, "package.json");

    return (await files.exists(path))
        ? Manifest.parse(JSON.parse(await files.readTextFile(path)))
        : undefined;
}

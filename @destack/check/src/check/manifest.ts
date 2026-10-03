import { readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { schema } from "@destack/schema";

/** The package.json fields checking reads: the dependencies selecting compiler settings and a workspace's members. */
export const Manifest = schema.looseObject({
    /** The package name. */
    name: schema.string().exactOptional(),
    /** The runtime dependencies. */
    dependencies: schema.record(schema.string(), schema.string()).exactOptional(),
    /** The development dependencies. */
    devDependencies: schema.record(schema.string(), schema.string()).exactOptional(),
    /** The member directory patterns of a workspace root, a `*` segment matching any directory. */
    workspaces: schema.array(schema.string()).exactOptional(),
});
/** The package.json fields checking reads. */
export type Manifest = schema.Infer<typeof Manifest>;

/** Read a directory's package manifest, absent without one. */
export async function readManifest(directory: string): Promise<Manifest | undefined> {
    const path = resolve(directory, "package.json");

    return existsSync(path) ? Manifest.parse(JSON.parse(await readFile(path, "utf8"))) : undefined;
}

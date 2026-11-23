import { fileURLToPath } from "node:url";
import { join } from "node:path";
import type { Environment } from "@destack/environment";
import { schema, Version } from "@destack/schema";
import { Release } from "@destack/update/release";
import { read } from "./command.ts";

/** The checkout the release tooling runs in, four directories above each source module. */
export const CHECKOUT = fileURLToPath(new URL("../../../..", import.meta.url));

/** The workspace manifest's field a release reads: its pinned Bun. */
export const WorkspaceManifest = schema.looseObject({
    /** The package manager the workspace pins, such as bun@1.4.3. */
    packageManager: schema.string().min(1),
});
/** The workspace manifest's fields a release reads. */
export type WorkspaceManifest = schema.Infer<typeof WorkspaceManifest>;

/** Read the workspace manifest at a checkout's root. */
export async function readWorkspace(
    root: string,
    host: Environment<"files">,
): Promise<WorkspaceManifest> {
    const text = await host.files.readTextFile(join(root, "package.json"));

    return WorkspaceManifest.parse(JSON.parse(text));
}

/** Read the version the distribution ships at: the release's, else dev. */
export function releaseVersion(host: Environment<"process" | "host">): string {
    // take the release's version, else dev
    const { os, architecture } = host.host;
    const given = host.process.variables["DESTACK_RELEASE_VERSION"] ?? Version.development;

    return new Release(given, Release.target(os, architecture)).version;
}

/** Read the Git commit a checkout has checked out. */
export async function readCommit(
    root: string,
    environment: Environment<"process">,
): Promise<string> {
    return (await read("git", ["rev-parse", "--verify", "HEAD"], environment, root)).trim();
}

/** Read the commit a release builds from, refusing a checkout with uncommitted changes. */
export async function readReleaseCommit(
    root: string,
    environment: Environment<"process">,
): Promise<string> {
    // refuse uncommitted changes, as every release is built from a commit
    const changed = await read("git", ["status", "--porcelain"], environment, root);
    if (changed.trim() !== "") {
        throw new Error(`cannot release '${root}': it has uncommitted changes, commit them first`);
    }

    return await readCommit(root, environment);
}

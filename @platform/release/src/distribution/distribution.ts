import { fileURLToPath } from "node:url";
import { join } from "node:path";
import type { Environment } from "@destack/environment";
import { schema } from "@destack/schema";
import { Release } from "@destack/update/release";
import { read } from "./command.ts";

/** The checkout the release tooling runs in, four directories above each source module. */
export const CHECKOUT = fileURLToPath(new URL("../../../..", import.meta.url));

/** The workspace manifest's fields a release reads: its version and its pinned Bun. */
export const WorkspaceManifest = schema.looseObject({
    /** The version every component of the distribution ships at. */
    version: schema.string().min(1),
    /** The package manager the workspace pins, such as bun@1.4.3. */
    packageManager: schema.string().min(1),
});
/** The workspace manifest's fields a release reads. */
export type WorkspaceManifest = schema.Infer<typeof WorkspaceManifest>;

/** The standalone commands each distribution ships in `bin` and beside its native desktop. */
export const COMMANDS = ["destack", "destack-machine", "destack-sandbox", "destack-build"];

/** Read the workspace manifest at a checkout's root. */
export async function readWorkspace(
    root: string,
    host: Environment<"files">,
): Promise<WorkspaceManifest> {
    const text = await host.files.readTextFile(join(root, "package.json"));

    return WorkspaceManifest.parse(JSON.parse(text));
}

/** Read the version every component of the distribution ships at, a release's override first. */
export async function releaseVersion(
    root: string,
    host: Environment<"files" | "process" | "host">,
): Promise<string> {
    // take the release's version, else the workspace's, for this host's target
    const manifest = await readWorkspace(root, host);
    const { os, architecture } = host.host;
    const given = host.process.variables["DESTACK_RELEASE_VERSION"] ?? manifest.version;

    return new Release(given, Release.target(os, architecture)).version;
}

/** Read the Git commit a checkout has checked out. */
export async function readCommit(
    root: string,
    environment: Environment<"process">,
): Promise<string> {
    return (await read("git", ["rev-parse", "--verify", "HEAD"], environment, root)).trim();
}

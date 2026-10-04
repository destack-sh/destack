import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { Release } from "@destack/update/release";
import { selectPlatform, version } from "../distribution/index.ts";
import { createSigner, signDistribution } from "./signer.ts";
import { run } from "../distribution/command.ts";

/** Repository containing the distribution build outputs. */
const ROOT = fileURLToPath(new URL("../../../../", import.meta.url));
/** The release whose built distribution this run unpacks or signs. */
const release = new Release(
    version,
    process.argv[3] ?? Release.target(process.platform, process.arch),
);
/** Compiled distribution selected for signing. */
const directory = join(ROOT, "dist", release.version, release.target);
/** Requested platform operation. */
const operation = process.argv[2];
/** The platform of the selected target. */
const platform = selectPlatform(release.target);

// unpack the archive a build job produced
if (operation === "unpack") {
    await mkdir(directory, { recursive: true });
    await run("tar", [
        "-xzf",
        join(ROOT, "dist", release.version, `destack-${release.directory}.tar.gz`),
        "-C",
        directory,
    ]);
}
// sign and notarize the exact application used by both the archive and the installers
else if (operation === "sign" && process.platform === platform.system) {
    if (process.env["DESTACK_SIGNING"] !== "release") {
        throw new Error("platform signing requires DESTACK_SIGNING=release");
    }
    const signer = createSigner(platform);
    if (signer === undefined) {
        throw new Error(`${release.target} has no platform signer`);
    }
    await signDistribution(directory, platform, signer);
}
// refuse every other operation
else {
    throw new Error("select unpack, or sign on the target operating system");
}

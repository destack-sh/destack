import { CHECKOUT } from "../distribution/distribution.ts";
import { join } from "node:path";
import { Release } from "@destack/update/release";
import { Platform, releaseVersion } from "../distribution/index.ts";
import type { Environment, HostCapability } from "@destack/environment";
import { Signer } from "./signer.ts";
import { run } from "../distribution/command.ts";

/** Unpack a built distribution, or sign it on its target operating system. */
export async function signPlatform(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // select the release of the requested target, this host's by default
    const version = await releaseVersion(CHECKOUT, environment);
    const [operation, selected] = arguments_;
    const release = new Release(
        version,
        selected ?? Release.target(environment.host.os, environment.host.architecture),
    );

    // select the compiled distribution and its platform
    const directory = join(CHECKOUT, "dist", release.version, release.target);
    const platform = Platform.of(release.target);

    // unpack the archive a build job produced
    if (operation === "unpack") {
        await environment.files.mkdir(directory, { recursive: true });
        await run(
            "tar",
            [
                "-xzf",
                join(CHECKOUT, "dist", release.version, `destack-${release.directory}.tar.gz`),
                "-C",
                directory,
            ],
            environment,
        );
    }
    // sign and notarize the exact application used by both the archive and the installers
    else if (operation === "sign" && environment.host.os === platform.system) {
        if (environment.process.variables["DESTACK_SIGNING"] !== "release") {
            throw new Error("platform signing requires DESTACK_SIGNING=release");
        }
        const signer = Signer.of(platform, environment);
        if (signer === undefined) {
            throw new Error(`${release.target} has no platform signer`);
        }
        await Signer.certify(directory, platform, signer, environment);
    }
    // refuse every other operation
    else {
        throw new Error("select unpack, or sign on the target operating system");
    }
}

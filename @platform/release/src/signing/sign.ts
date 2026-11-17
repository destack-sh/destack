import { CHECKOUT } from "../distribution/distribution.ts";
import { join } from "node:path";
import { Release } from "@destack/update/release";
import { Installer, PLATFORMS, releaseVersion } from "../distribution/index.ts";
import type { Environment, HostCapability } from "@destack/environment";
import { Signer } from "./signer.ts";
import { run } from "../distribution/command.ts";
import { checkDistribution } from "../distribution/check.ts";
import { buildFormat } from "../installer/build.ts";
import { print } from "../distribution/output.ts";
import { present } from "@destack/schema";

/** Unpack a built distribution, or sign this system's distributions with their installers. */
export async function signPlatform(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // select the release of the requested target, this host's by default
    const version = releaseVersion(environment);
    const [operation, selected] = arguments_;
    const release = new Release(
        version,
        selected ?? Release.target(environment.host.os, environment.host.architecture),
    );

    // select the compiled distribution
    const directory = join(CHECKOUT, "dist", release.version, release.target);

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
    // sign the exact applications the archives and the installers ship
    else if (operation === "sign" && selected === undefined) {
        if (environment.process.variables["DESTACK_SIGNING"] !== "release") {
            throw new Error("platform signing requires DESTACK_SIGNING=release");
        }
        const { os } = environment.host;
        const signed = PLATFORMS.filter((each) => each.signer !== undefined && each.system === os);
        const applications: Record<string, string> = {};
        for (const each of signed) {
            const signer = present(Signer.of(each, environment), `the signer of ${each.target}`);
            const built = join(CHECKOUT, "dist", version, each.target);
            await Signer.certify(built, each, signer, environment);
            applications[each.target] = join(built, each.bundle.application);
        }

        // refuse a signed platform no installer ships, as its installer notarizes it
        const shipped = Installer.list().filter(({ platforms }) =>
            platforms.some((each) => signed.includes(each)),
        );
        const unshipped = signed.find(
            (each) => !shipped.some(({ platforms }) => platforms.includes(each)),
        );
        if (unshipped !== undefined) {
            throw new Error(`'${unshipped.target}' has a signer and no installer to notarize it`);
        }

        // build each installer, notarizing its application with the signed ones in one submission
        for (const { installer } of shipped) {
            print(await buildFormat(installer, environment, applications), environment);
        }

        // run each signed distribution, as signing and the hardened runtime can stop it from starting
        for (const each of signed) {
            const built = join(CHECKOUT, "dist", version, each.target);
            await checkDistribution(built, each.target, version, environment);
        }
    }
    // refuse every other operation
    else {
        throw new Error("select unpack with a target, or sign without one");
    }
}

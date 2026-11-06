import { CHECKOUT } from "./distribution.ts";
import { join } from "node:path";
import { Release } from "@destack/update/release";
import type { Environment, HostCapability } from "@destack/environment";
import { releaseVersion } from "./index.ts";

/** Open the compiled desktop of this host's target and exit with its exit code. */
export async function openDistribution(environment: Environment<HostCapability>): Promise<void> {
    // select the compiled distribution of this host's target
    const version = await releaseVersion(CHECKOUT, environment);
    const { os, architecture } = environment.host;
    const directory = join(CHECKOUT, "dist", version, Release.target(os, architecture));

    // select the native desktop entrypoint inside it
    const executable =
        os === "darwin"
            ? join(directory, "Destack.app/Contents/MacOS/Destack")
            : join(directory, "Destack", "Destack");

    // run the desktop interactively
    const child = environment.process.spawn([executable], {
        stdin: "inherit",
        stdout: "inherit",
        stderr: "inherit",
    });
    environment.process.exitCode = await child.exited;
}

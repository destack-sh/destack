import { join } from "node:path";
import { Digest } from "@destack/schema";
import { SandboxTool } from "@destack/sandbox";
import type { Environment, HostCapability } from "@destack/environment";
import { run } from "./command.ts";

/** The ripgrep the Linux sandbox scans a workload's directory with, its latest stable release. */
const RIPGREP_VERSION = "15.2.0";

/** The socat source the Linux sandbox's proxy bridge builds from, pinned by its SHA-256. */
const SOCAT_SOURCE = {
    name: "socat-1.8.1.3",
    url: "http://www.dest-unreach.org/socat/download/socat-1.8.1.3.tar.gz",
    digest: "06602ffd591e98c75b3dc1d66f0f19136cc666b0b2d95caad987d6ab2cb28097",
} as const;

/** How long the socat source may take to download: a minute for its 750 KB. */
const DOWNLOAD_TIMEOUT_MILLISECONDS = 60_000;

/** Build the Linux sandbox's tools into a release's helpers, against its desktop's C library. */
export async function buildSandboxTools(
    helpers: string,
    scratch: string,
    environment: Environment<HostCapability>,
): Promise<void> {
    // build ripgrep from its crate, stripped of the debug information its release profile keeps
    const { files } = environment;
    const cargo = join(scratch, "ripgrep");
    await run(
        "cargo",
        ["install", "ripgrep", "--version", RIPGREP_VERSION, "--locked", "--root", cargo],
        environment,
        scratch,
        { CARGO_PROFILE_RELEASE_STRIP: "symbols" },
    );
    await files.rename(join(cargo, "bin", "rg"), join(helpers, SandboxTool.file("ripgrep")));

    // download socat's source, refusing any bytes but the pinned ones
    const response = await environment.network.fetch(
        new Request(SOCAT_SOURCE.url, {
            redirect: "error",
            signal: environment.clock.timeout(DOWNLOAD_TIMEOUT_MILLISECONDS),
        }),
    );
    if (!response.ok) {
        throw new Error(`socat source download failed: HTTP ${response.status}`);
    }
    const bytes = new Uint8Array(await response.arrayBuffer());
    const digest = await Digest.of(bytes, environment.crypto);
    if (digest !== SOCAT_SOURCE.digest) {
        throw new Error(`socat source has digest ${digest}, not ${SOCAT_SOURCE.digest}`);
    }

    // build socat without OpenSSL, readline or libwrap, so it needs the C library alone
    const archive = join(scratch, `${SOCAT_SOURCE.name}.tar.gz`);
    const source = join(scratch, SOCAT_SOURCE.name);
    await files.writeFile(archive, bytes);
    await run("tar", ["-xzf", archive, "-C", scratch], environment);
    const options = ["--disable-openssl", "--disable-readline", "--disable-libwrap"];
    await run("./configure", options, environment, source);
    await run("make", ["socat"], environment, source);
    await files.rename(join(source, "socat"), join(helpers, SandboxTool.file("socat")));
}

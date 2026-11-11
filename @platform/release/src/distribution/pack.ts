import { CHECKOUT } from "./distribution.ts";
import { createHash } from "node:crypto";
import { join } from "node:path";
import type { Environment, HostCapability } from "@destack/environment";
import { Platform, releaseVersion } from "./index.ts";
import { Release } from "@destack/update/release";
import { print } from "./output.ts";

/** Package one compiled target and record its digest. */
export async function packDistribution(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // package the desktop and standalone CLI together
    const { files, host, process } = environment;
    const version = await releaseVersion(CHECKOUT, environment);
    const [selected] = arguments_;
    const target = new Release(version, selected ?? Release.target(host.os, host.architecture))
        .target;
    const directory = join(CHECKOUT, "dist", version);
    const name = `destack-${version}-${target}.tar.gz`;
    const child = process.spawn(
        [
            "tar",
            "-czf",
            join(directory, name),
            "-C",
            join(directory, target),
            Platform.of(target).bundle.application,
            "bin",
        ],
        {
            variables: { ...process.variables, COPYFILE_DISABLE: "1" },
            stdout: "inherit",
            stderr: "inherit",
        },
    );
    const code = await child.exited;
    if (code !== 0) {
        throw new Error(`archive creation failed: '${code}'`);
    }

    // record the exact bytes published for this target
    const hash = createHash("sha256");
    let size = 0;
    await using archive = await files.open(join(directory, name), { read: true });
    for await (const bytes of archive.readable()) {
        hash.update(bytes);
        size += bytes.length;
    }
    const sha256 = hash.digest("hex");

    // persist the digest and size from the same bounded read
    await files.writeTextFile(
        join(directory, `${target}.json`),
        JSON.stringify(
            {
                target,
                file: name,
                size,
                sha256,
            },
            null,
            4,
        ) + "\n",
    );
    print(`${name}: ${size} bytes, SHA-256 ${sha256}`, environment);
}

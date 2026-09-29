import { cp, mkdir } from "node:fs/promises";
import { join } from "node:path";
import { Release } from "@destack/update/release";
import { version } from "../distribution/index.ts";
import { MacSigning } from "./apple.ts";
import { run } from "../distribution/command.ts";

/** Unpack or sign one built platform distribution. */
const release = new Release(version, process.argv[3] ?? Release.target());
/** Compiled distribution selected for signing. */
const directory = join("dist", release.version, release.target);
/** Requested platform operation. */
const operation = process.argv[2];
if (operation === "unpack") {
    await mkdir(directory, { recursive: true });
    await run("tar", [
        "-xzf",
        join("dist", release.version, `destack-${release.directory}.tar.gz`),
        "-C",
        directory,
    ]);
}
// notarize the exact application used by both the archive and disk image
else if (
    operation === "sign" &&
    process.platform === "darwin" &&
    release.target.endsWith("apple-darwin")
) {
    if (process.env.DESTACK_SIGNING !== "release") {
        throw new Error("platform signing requires DESTACK_SIGNING=release");
    }
    const signing = new MacSigning();
    const application = join(directory, "Destack.app");
    await signing.application(application);
    for (const name of ["destack", "destack-daemon", "destack-sandbox"]) {
        await cp(join(application, "Contents/Helpers", name), join(directory, "bin", name));
    }
    await signing.notarizeApplication(application);
} else {
    throw new Error("select unpack, or sign on macOS");
}

import { homedir, hostname, release } from "node:os";
import type { Architecture, Host, OperatingSystem } from "./host.ts";

/** The operating systems Bun runs on. */
const OPERATING_SYSTEMS: readonly OperatingSystem[] = ["darwin", "linux", "win32"];

/** The processor architectures Bun runs on. */
const ARCHITECTURES: readonly Architecture[] = ["arm64", "x64"];

/** The machines Bun runs on. */
export const BunHost = {
    /** Read the machine Bun runs on, refusing one outside Bun's targets. */
    read(): Host {
        // refuse a machine outside Bun's targets
        const os = OPERATING_SYSTEMS.find((known) => known === process.platform);
        const architecture = ARCHITECTURES.find((known) => known === process.arch);
        if (os === undefined || architecture === undefined) {
            throw new TypeError(`bun does not run on ${process.platform} ${process.arch}`);
        }

        return { os, architecture, version: release(), hostname: hostname(), home: homedir() };
    },
};

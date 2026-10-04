import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import type { Metadata, Targets } from "@tufjs/models";
import { Release } from "@destack/update/release";
import { TargetCustom } from "@destack/update/release";
import type { Catalog, CatalogDistribution } from "./catalog.ts";
import { INSTALLER_FORMATS, InstallerFormat } from "./catalog.ts";

/** A signed target path: a target or installer name, then the update archive or an installer format. */
const TARGET_PATH = new RegExp(`^([a-z0-9_-]+)\\.(tar\\.gz|${INSTALLER_FORMATS.join("|")})$`, "u");

/** Generate download links and bootstrap installers from signed target metadata. */
export async function writeBootstrap(
    directory: string,
    targets: Metadata<Targets>,
    url: URL,
): Promise<void> {
    // derive downloads and bootstrap URLs from the signed target descriptions
    const downloads: Record<string, CatalogDistribution> = {};
    const cases: string[] = [];
    const versions = new Set<string>();
    for (const [path, file] of Object.entries(targets.signed.targets)) {
        const match = TARGET_PATH.exec(path);
        const target = match?.[1];
        if (!match || target === undefined) {
            throw new Error(`invalid target path: ${path}`);
        }
        const sha256 = file.hashes["sha256"];
        if (sha256 === undefined || !/^[a-f0-9]{64}$/u.test(sha256)) {
            throw new Error("invalid distribution digest");
        }
        const download = new URL(`targets/${sha256}.${path}`, url).href;
        const { version } = TargetCustom.parse(file.unrecognizedFields["custom"]);
        const distribution = (downloads[target] ??= { installers: {} });
        const entry = { version, url: download, sha256, size: file.length };
        versions.add(version);

        // list an updater archive and its installer case
        const format = match[2];
        if (format === "tar.gz") {
            distribution.archive = entry;
            cases.push(`    ${target}/${format}) url='${download}'; sha256='${sha256}' ;;`);
        }
        // list a native installer under its format
        else {
            distribution.installers[InstallerFormat.parse(format)] = entry;
        }
    }

    // publish only a complete single-version selection
    const [version] = versions;
    if (versions.size !== 1 || version === undefined) {
        throw new Error("catalog requires one release version across all targets");
    }
    const release = new Release(version, "aarch64-apple-darwin");
    const catalog: Catalog = { version, channel: release.channel, downloads };

    // publish the catalog for browser selection and the installers for terminal setup
    await writeFile(join(directory, "downloads.json"), JSON.stringify(catalog, null, 4) + "\n");
    const shell = await readFile(new URL("../installer/install.sh", import.meta.url), "utf8");
    await writeFile(
        join(directory, "install"),
        shell.replace("# __DISTRIBUTIONS__", cases.join("\n")),
    );
}

import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import type { Metadata, Targets } from "@tufjs/models";
import { Release } from "@destack/update/release";

/** Public download choices derived from one signed release. */
export interface Catalog {
    /** Calendar version shared by every download. */
    version: string;
    /** Stable or nightly distribution selection. */
    channel: "stable" | "nightly";
    /** Platform-specific archives and the universal macOS installer. */
    downloads: Record<string, CatalogDistribution>;
}

/** An updater archive and native installer choices for one platform. */
export interface CatalogDistribution {
    /** Complete archive consumed by the authenticated updater. */
    archive?: CatalogDownload;
    /** Native installers keyed by their file format. */
    installers: Partial<Record<"dmg" | "exe", CatalogDownload>>;
}

/** One public archive or installer authenticated by targets metadata. */
export interface CatalogDownload {
    /** Calendar version reported by the installed programs. */
    version: string;
    /** Immutable HTTPS artifact URL. */
    url: string;
    /** Lowercase SHA-256 artifact digest. */
    sha256: string;
    /** Exact archive length in bytes. */
    size: number;
}

/** Generate download links and bootstrap installers from signed target metadata. */
export async function writeCatalog(
    directory: string,
    targets: Metadata<Targets>,
    url: URL,
): Promise<void> {
    // derive downloads and bootstrap URLs from the signed target descriptions
    const downloads: Record<string, CatalogDistribution> = {};
    const cases: string[] = [];
    const versions = new Set<string>();
    for (const [path, file] of Object.entries(targets.signed.targets)) {
        const match = /^([a-z0-9_-]+)\.(tar\.gz|dmg|exe)$/.exec(path);
        if (!match) {
            throw new Error(`invalid target path: ${path}`);
        }
        const target = match[1];
        const sha256 = file.hashes.sha256;
        if (!/^[a-f0-9]{64}$/.test(sha256)) {
            throw new Error("invalid distribution digest");
        }
        const download = new URL(`targets/${sha256}.${path}`, url).href;
        const version = (file.unrecognizedFields.custom as { version: string }).version;
        const distribution = (downloads[target] ??= { installers: {} });
        const entry = { version, url: download, sha256, size: file.length };
        versions.add(version);
        if (match[2] === "tar.gz") {
            distribution.archive = entry;
        }
        // retain every native format without replacing its platform's updater archive
        else {
            distribution.installers[match[2] as "dmg" | "exe"] = entry;
        }
        if (match[2] === "tar.gz") {
            cases.push(`    ${target}/${match[2]}) url='${download}'; sha256='${sha256}' ;;`);
        }
    }

    // publish only a complete single-version selection
    if (versions.size !== 1) {
        throw new Error("catalog requires one release version across all targets");
    }
    const version = [...versions][0]!;
    const release = new Release(version, "aarch64-apple-darwin");
    const catalog: Catalog = { version, channel: release.channel, downloads };

    // publish the catalog for browser selection and the installers for terminal setup
    await writeFile(join(directory, "downloads.json"), JSON.stringify(catalog, null, 4) + "\n");
    const shell = await readFile(new URL("../installer/install.sh", import.meta.url), "utf8");
    const powershell = await readFile(new URL("../installer/install.ps1", import.meta.url), "utf8");
    await writeFile(
        join(directory, "install"),
        shell.replace("# __DISTRIBUTIONS__", cases.join("\n")),
    );
    await writeFile(
        join(directory, "install.ps1"),
        powershell.replaceAll("__REPOSITORY__", url.href),
    );
}

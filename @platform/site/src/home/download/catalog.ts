import type { CatalogDownload } from "@platform/release/catalog";
import { readFields, readString } from "../../content/json.ts";

/** The platform name of each published desktop distribution target. */
const LABELS = {
    "universal-apple-darwin": "macOS",
    "aarch64-apple-darwin": "macOS · Apple Silicon",
    "x86_64-apple-darwin": "macOS · Intel",
    "x86_64-unknown-linux-gnu": "Linux · x64",
    "aarch64-unknown-linux-gnu": "Linux · ARM64",
};

/** The operating systems the desktop distribution targets, as one phrase. */
export const systems = [...new Set(Object.values(LABELS).map(systemOf))].join(" and ");

/** A published desktop distribution target. */
type Target = keyof typeof LABELS;

/** A desktop distribution in the public release catalog. */
export interface Download {
    /** Operating system and architecture. */
    target: Target;
    /** Human readable platform name. */
    label: string;
    /** Published application version. */
    version: string;
    /** Immutable download address. */
    url: string;
}

/** Read and validate the public download catalog. */
export async function readDownloads(): Promise<Download[]> {
    // fetch the catalog and reject a failed response or shape
    const response = await fetch("https://download.destack.sh/downloads.json");
    if (!response.ok) {
        throw new Error(`download catalog returned ${response.status}`);
    }
    const catalog = readFields(await response.json(), "download catalog");
    const distributions = readFields(catalog.get("downloads"), "download catalog downloads");

    // offer the Linux archive and the native installers, each at a published address
    const downloads = [...distributions].flatMap(([target, value]) => {
        if (!isTarget(target)) {
            throw new Error(`unknown download platform: ${target}`);
        }
        const distribution = readFields(value, `download platform ${target}`);
        const installers = readFields(distribution.get("installers"), `${target} installers`);
        const isLinux = target.endsWith("unknown-linux-gnu");
        const archive = isLinux ? distribution.get("archive") : undefined;
        const choice = archive ?? installers.get("dmg");

        return choice === undefined ? [] : [downloadOf(target, choice)];
    });
    if (downloads.length === 0) {
        throw new Error("no downloads published");
    }

    // offer one Mac download when the universal installer is published
    const isUniversal = downloads.some((download) => download.target === "universal-apple-darwin");

    return isUniversal
        ? downloads.filter(
              (download) =>
                  download.target !== "aarch64-apple-darwin" &&
                  download.target !== "x86_64-apple-darwin",
          )
        : downloads;
}

/** Select only platforms whose architecture is known or universal. */
export function selectDownload(downloads: Download[], agent: string): Download | undefined {
    if (/Android|iPhone|iPad|Mobile/u.test(agent)) {
        return undefined;
    }
    if (/Macintosh/u.test(agent)) {
        return downloads.find((download) => download.target === "universal-apple-darwin");
    }
    if (/Linux/u.test(agent) && /x86_64/u.test(agent)) {
        return downloads.find((download) => download.target === "x86_64-unknown-linux-gnu");
    }

    return undefined;
}

/** Read one catalog choice as a download, refusing an address outside the release targets. */
function downloadOf(target: Target, value: unknown): Download {
    // read the address and version of the release's catalog download
    const fields = readFields(value, `download ${target}`);
    const url: CatalogDownload["url"] = readString(fields.get("url"), `download ${target} url`);
    const version: CatalogDownload["version"] = readString(
        fields.get("version"),
        `download ${target} version`,
    );

    // require a calendar version
    if (!/^\d{4}\.\d+\.\d+(?:-nightly\.\d+)?$/u.test(version)) {
        throw new Error(`invalid download version: ${version}`);
    }

    // require an immutable target address on the download origin
    const address = new URL(url);
    if (
        address.origin !== "https://download.destack.sh" ||
        address.search !== "" ||
        address.hash !== "" ||
        !/^\/(?:stable|nightly)\/targets\/[a-f0-9]{64}\.[a-z0-9_-]+\.(?:dmg|tar\.gz)$/u.test(
            address.pathname,
        )
    ) {
        throw new Error("invalid download address");
    }

    return { target, label: LABELS[target], version, url };
}

/** Report whether a catalog key names a published desktop distribution target. */
function isTarget(key: string): key is Target {
    return Object.hasOwn(LABELS, key);
}

/** Return the operating system a platform label names, such as macOS for "macOS · Intel". */
export function systemOf(label: string) {
    return label.split(" · ")[0];
}

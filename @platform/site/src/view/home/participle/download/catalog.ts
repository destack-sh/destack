import { schema } from "@destack/schema";

/** One public archive or installer in the download catalog. */
const CatalogDownload = schema.object({
    /** Calendar version reported by the installed programs. */
    version: schema.string(),
    /** Immutable HTTPS artifact URL. */
    url: schema.string(),
    /** Lowercase SHA-256 artifact digest. */
    sha256: schema.string(),
    /** Exact archive length in bytes. */
    size: schema.number(),
});

/** One public archive or installer in the download catalog. */
type CatalogDownload = schema.Infer<typeof CatalogDownload>;

// NOTE #Architecture: mirrors @platform/release's Catalog so the site skips release's build closure
/** The public download choices of one signed release. */
const Catalog = schema.object({
    /** Calendar version shared by every download. */
    version: schema.string(),
    /** Platform-specific archives and installers, by target or installer name. */
    downloads: schema.record(
        schema.string(),
        schema.object({
            /** Complete archive consumed by the authenticated updater. */
            archive: CatalogDownload.exactOptional(),
            /** Native installers keyed by their file format. */
            installers: schema.partialRecord(schema.enum(["dmg"]), CatalogDownload),
        }),
    ),
});

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

/** An operating system a visitor's browser runs on, which the desktop app may have a build for. */
export type System = "macOS" | "Windows" | "Linux";

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
    const catalog = Catalog.parse(await response.json());

    // offer the Linux archive and the native installers, each at a published address
    const downloads = Object.entries(catalog.downloads).flatMap(([target, distribution]) => {
        if (!isTarget(target)) {
            throw new Error(`unknown download platform: ${target}`);
        }
        const isLinux = target.endsWith("unknown-linux-gnu");
        const choice = isLinux ? distribution.archive : distribution.installers.dmg;

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

/** Read the desktop operating system from a user agent, or none on a phone or tablet. */
export function systemOfAgent(agent: string): System | undefined {
    // skip phones and tablets, which run no desktop app
    if (/Android|iPhone|iPad|Mobile/u.test(agent)) {
        return undefined;
    }
    // match the desktop systems by their user agent tokens
    else if (/Macintosh/u.test(agent)) {
        return "macOS";
    } else if (/Windows/u.test(agent)) {
        return "Windows";
    } else if (/Linux/u.test(agent)) {
        return "Linux";
    }
    // leave any other system unknown
    else {
        return undefined;
    }
}

/** Read one catalog choice as a download, refusing an address outside the release targets. */
function downloadOf(target: Target, download: CatalogDownload): Download {
    // read the address and version of the release's catalog download
    const { url, version } = download;

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

/** Report whether a catalog key is a published desktop distribution target. */
function isTarget(key: string): key is Target {
    return Object.hasOwn(LABELS, key);
}

/** Return the operating system in a platform label, such as macOS for "macOS · Intel". */
export function systemOf(label: string) {
    return label.split(" · ")[0];
}

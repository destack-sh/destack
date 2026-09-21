/** Published desktop distributions. */
export const platforms = {
    "universal-apple-darwin": "macOS",
    "aarch64-apple-darwin": "macOS · Apple Silicon",
    "x86_64-apple-darwin": "macOS · Intel",
    "x86_64-pc-windows-msvc": "Windows",
    "x86_64-unknown-linux-gnu": "Linux · x64",
    "aarch64-unknown-linux-gnu": "Linux · ARM64",
} as const;

/** A desktop distribution in the public release catalog. */
export interface Download {
    /** Operating system and architecture. */
    target: keyof typeof platforms;
    /** Human readable platform name. */
    label: string;
    /** Published application version. */
    version: string;
    /** Immutable download address. */
    url: string;
}

/** Read and validate the public download catalog. */
export async function readDownloads(): Promise<Download[]> {
    const response = await fetch("https://download.destack.sh/downloads.json");
    if (!response.ok) {
        throw new Error(`Download catalog returned ${response.status}.`);
    }
    const catalog = await response.json();

    // validate addresses before placing links in the page
    const downloads = Object.entries(catalog.downloads).map(([target, value]) => {
        const { url, version } = value as { url: string; version: string };
        if (
            !(target in platforms) ||
            typeof url !== "string" ||
            typeof version !== "string" ||
            !/^\d{4}\.\d+\.\d+$/.test(version)
        ) {
            throw new Error("invalid download catalog");
        }
        const address = new URL(url);
        if (
            address.origin !== "https://download.destack.sh" ||
            address.search ||
            address.hash ||
            !/^\/targets\/[a-f0-9]{64}\.[a-z0-9_-]+\.(tar\.gz|dmg|exe)$/.test(address.pathname)
        ) {
            throw new Error("invalid download address");
        }

        return {
            target: target as Download["target"],
            label: platforms[target as Download["target"]],
            version,
            url,
        };
    });
    if (!downloads.length) {
        throw new Error("no downloads published");
    }

    // offer one Mac download when the universal installer is published
    return downloads.some((download) => download.target === "universal-apple-darwin")
        ? downloads.filter(
              (download) =>
                  !["aarch64-apple-darwin", "x86_64-apple-darwin"].includes(download.target),
          )
        : downloads;
}

/** Select only platforms whose architecture is known or universal. */
export function selectDownload(downloads: Download[], agent: string): Download | undefined {
    if (/Android|iPhone|iPad|Mobile/.test(agent)) {
        return undefined;
    }
    if (/Macintosh/.test(agent)) {
        return downloads.find((download) => download.target === "universal-apple-darwin");
    }
    if (/Windows/.test(agent) && /Win64|x64/.test(agent)) {
        return downloads.find((download) => download.target === "x86_64-pc-windows-msvc");
    }
    if (/Linux/.test(agent) && /x86_64/.test(agent)) {
        return downloads.find((download) => download.target === "x86_64-unknown-linux-gnu");
    }

    return undefined;
}

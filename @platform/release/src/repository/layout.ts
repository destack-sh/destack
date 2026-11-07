/** The paths of a public release repository: metadata, targets and bootstrap files. */
export const PUBLIC_PATH =
    /^(?:metadata\/(?:[1-9]\d*\.(?:root|targets|snapshot)|timestamp)\.json|targets\/[0-9a-f]{64}\.[a-z0-9_-]+\.(?:tar\.gz|dmg)|downloads\.json|install)$/u;

/** The bootstrap files the download domain serves at its root, beside the feed's own path. */
export const ROOT_FILES: ReadonlySet<string> = new Set(["install", "downloads.json"]);

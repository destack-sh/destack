/** The paths of a public release repository: metadata, targets, blobs and bootstrap files. */
export const PUBLIC_PATH =
    /^(?:metadata\/(?:[1-9]\d*\.(?:root|targets|snapshot)|timestamp)\.json|targets\/[0-9a-f]{64}\.[a-z0-9_-]+\.(?:json|tar\.gz|dmg)|targets\/blobs\/[0-9a-f]{64}|downloads\.json|install)$/u;

/** The paths a public release repository holds: signed metadata, content-addressed targets and bootstrap files. */
export const PUBLIC_PATH =
    /^(?:metadata\/(?:[1-9]\d*\.(?:root|targets|snapshot)|timestamp)\.json|targets\/[0-9a-f]{64}\.[a-z0-9_-]+\.(?:tar\.gz|dmg)|downloads\.json|install)$/u;

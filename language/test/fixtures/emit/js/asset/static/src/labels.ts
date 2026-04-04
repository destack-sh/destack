/** Build one image label string for the static asset fixture. */
export function buildImageLabel(name: string) {
    return `asset:${name}`;
}

/** Build a small asset manifest string for the static asset fixture. */
export function buildImageManifest(assets) {
    const assetNames = Object.keys(assets).sort();
    const assetList = assetNames
        .map((name) => `${name}=${assets[name]}`)
        .join(",");

    return `manifest:${assetList}`;
}

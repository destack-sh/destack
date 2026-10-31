import { ManifestOptions } from "./site.ts";

/** Write a web app manifest with the member names the specification spells. */
export function writeManifest(options: ManifestOptions): string {
    const manifest = {
        name: options.name,
        short_name: options.shortName,
        description: options.description,
        start_url: options.startUrl ?? "/",
        display: options.display,
        background_color: options.backgroundColor,
        theme_color: options.themeColor,
        icons: options.icons,
    };

    return `${JSON.stringify(manifest, null, 4)}\n`;
}

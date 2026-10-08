import { schema } from "@destack/schema";

/** A site's web app manifest, per the W3C Web App Manifest. */
export const ManifestOptions = schema.object({
    /** The app's name. */
    name: schema.string().min(1),
    /** The name where space is short, such as under a home screen icon. */
    shortName: schema.string().exactOptional(),
    /** What the app is for. */
    description: schema.string().exactOptional(),
    /** The page the app opens at, `/` by default. */
    startUrl: schema.string().exactOptional(),
    /** How the app shows when installed. */
    display: schema.enum(["fullscreen", "standalone", "minimal-ui", "browser"]).exactOptional(),
    /** The splash screen's background color. */
    backgroundColor: schema.string().exactOptional(),
    /** The browser interface's color. */
    themeColor: schema.string().exactOptional(),
    /** The app's icons. */
    icons: schema
        .array(
            schema.object({
                /** The icon's address. */
                src: schema.string().min(1),
                /** The sizes, such as `192x192` or `any`. */
                sizes: schema.string().min(1),
                /** The media type, such as `image/png`. */
                type: schema.string().min(1),
                /** Where the icon fits, such as `maskable`. */
                purpose: schema.string().exactOptional(),
            }),
        )
        .readonly(),
});
/** A site's web app manifest. */
export type ManifestOptions = schema.Infer<typeof ManifestOptions>;

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

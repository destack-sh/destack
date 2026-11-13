import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { frame } from "@destack/style/frame.stylex";
import { DEFAULT_DISPLAY } from "@destack/theme";
import type { ParentProps } from "@destack/view";
import { AppearanceScript, destackFonts, Font, Viewport } from "@destack/view/document";
import { HydrationScript } from "@destack/view/static";
import { skyScript } from "./effect/sky";
import { destackTheme } from "@destack/theme/declare";

/** The theme root's custom properties, following the system appearance until one is picked. */
const themeStyle = destackTheme.variables(DEFAULT_DISPLAY);

/** The site's frame: four columns on phones, twelve from tablets, ruled cells without gutters. */
const siteFrame = style.createTheme(frame, {
    columns: { default: "4", [media.md]: "12" },
    margin: { default: "0px", [media.md]: "24px" },
    gutter: "0px",
});

/** The page's root styles: clip sideways overflow and keep the page from bouncing past its ends. */
const styles = style.create({
    page: {
        maxWidth: "100%",
        overflowX: "clip",
        overscrollBehavior: "none",
    },
});

/** Render the shared HTML document. */
export default function Document(properties: ParentProps) {
    return (
        <html lang="en" {...style.attributes([styles.page, siteFrame], themeStyle)}>
            <head>
                <meta charset="utf-8" />
                <Viewport colorScheme="light dark" />
                <AppearanceScript />
                <script innerHTML={skyScript} />
                {destackFonts.map((font) => (
                    <Font font={font} />
                ))}
                <HydrationScript />
            </head>
            <body {...style.attrs(styles.page)}>
                <div id="app">{properties.children}</div>
            </body>
        </html>
    );
}

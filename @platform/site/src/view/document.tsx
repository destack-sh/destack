import * as style from "@destack/style";
import { DEFAULT_PREFERENCES } from "@destack/theme";
import type { ParentProps } from "@destack/view";
import { AppearanceScript, Font, Viewport } from "@destack/view/document";
import { HydrationScript } from "@destack/web/render";
import "@destack/view/styles";
import { mono, sans, sansItalics } from "./font.ts";
import { theme } from "./theme.ts";

/** The theme root's custom properties, following the system's appearance until a visitor picks one. */
const themeStyle = theme.variables("system", DEFAULT_PREFERENCES);

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
        <html lang="en" {...style.attributes(styles.page, themeStyle)}>
            <head>
                <meta charset="utf-8" />
                <Viewport colorScheme="light dark" />
                <AppearanceScript />
                <Font font={sans} />
                <Font font={sansItalics} />
                <Font font={mono} />
                <HydrationScript />
            </head>
            <body {...style.attrs(styles.page)}>
                <div id="app">{properties.children}</div>
            </body>
        </html>
    );
}

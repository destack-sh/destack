import * as style from "@destack/style";
import { SOURCE_LOCALE } from "@destack/locale";
import { DEFAULT_DISPLAY, Theme, type ThemeDescription } from "@destack/theme";
import { destackTheme } from "@destack/theme/declare";
import { HydrationScript, type JSX } from "@solidjs/web";
import { AppearanceScript, destackFonts, Font, Viewport } from "../document/index.ts";

/** What a static view's default page shell renders: the page under the package's theme. */
export interface ViewDocumentProperties {
    /** The page. */
    readonly children?: JSX.Element;
    /** The theme the package declares, Destack's when absent. */
    readonly theme?: ThemeDescription;
}

/** Render a static view's page shell, styled by the package's theme and hydrating the page. */
export function ViewDocument(properties: ViewDocumentProperties): JSX.Element {
    // style the root by the package's theme in the default display, following the visitor's appearance
    const theme =
        properties.theme === undefined
            ? destackTheme
            : new Theme(properties.theme.package, properties.theme);
    const variables = theme.variables(DEFAULT_DISPLAY);

    return (
        <html lang={SOURCE_LOCALE} {...style.attributes([], variables)}>
            <head>
                <meta charset="utf-8" />
                <Viewport colorScheme="light dark" />
                <AppearanceScript />
                {destackFonts.map((font) => (
                    <Font font={font} />
                ))}
                <HydrationScript />
            </head>
            <body>{properties.children}</body>
        </html>
    );
}

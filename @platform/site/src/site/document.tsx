import type { ParentProps } from "@destack/view";
import { HydrationScript } from "@destack/view/render";
import { createTheme } from "@destack/theme";
import ibmPlexMono from "@fontsource/ibm-plex-mono/files/ibm-plex-mono-latin-400-normal.woff2?url";
import ibmPlexMonoSemibold from "@fontsource/ibm-plex-mono/files/ibm-plex-mono-latin-600-normal.woff2?url";
import ibmPlexSans from "@fontsource-variable/ibm-plex-sans/files/ibm-plex-sans-latin-wght-normal.woff2?url";
import ibmPlexSansCondensed from "@fontsource/ibm-plex-sans-condensed/files/ibm-plex-sans-condensed-latin-500-normal.woff2?url";
import limelight from "@fontsource/limelight/files/limelight-latin-400-normal.woff2?url";
import "@destack/theme/theme.css";

/** Publication typography and neutral palette. */
const theme = createTheme({
    gray: "sand",
    accent: "orange",
    fontFamily: '"IBM Plex Sans Variable", Helvetica, Arial, sans-serif',
    monospaceFontFamily: '"IBM Plex Mono", ui-monospace, SFMono-Regular, Menlo, monospace',
});

// map the paper and night palettes onto the shared semantic roles
Object.assign(theme.style, {
    "--destack-color-background": "light-dark(#f8f5ee, #0b2029)",
    "--destack-color-foreground": "light-dark(#12313c, #f1eadb)",
    "--destack-color-muted": "light-dark(#eeebe3, #122a33)",
    "--destack-color-mutedForeground": "light-dark(#5c6a6d, #a8bbbd)",
    "--destack-color-border": "light-dark(#12313c2e, #f1eadb33)",
    "--destack-color-primary": "#ff792e",
});

/** Render the shared HTML document. */
export default function Document(properties: ParentProps) {
    return (
        <html lang="en" {...theme}>
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                {/* load extracted styles before rendering the document */}
                {import.meta.env.DEV && (
                    <>
                        <link rel="stylesheet" href="/virtual:stylex.css" />
                        <script type="module" src="/@id/virtual:stylex:runtime" />
                    </>
                )}
                <script src="/theme.js" />
                <link rel="icon" href="/brand/favicon/favicon.svg" type="image/svg+xml" />
                <link
                    rel="icon"
                    href="/brand/favicon/favicon-32.png"
                    type="image/png"
                    sizes="32x32"
                />
                <link rel="apple-touch-icon" href="/brand/icon/icon-180.png" />
                {/* start the visible heading, navigation, and reading fonts with the document */}
                {[
                    limelight,
                    ibmPlexMono,
                    ibmPlexMonoSemibold,
                    ibmPlexSans,
                    ibmPlexSansCondensed,
                ].map((font) => (
                    <link
                        rel="preload"
                        as="font"
                        type="font/woff2"
                        crossorigin="anonymous"
                        href={font}
                    />
                ))}
                <HydrationScript />
            </head>
            <body>
                <div id="app">{properties.children}</div>
            </body>
        </html>
    );
}

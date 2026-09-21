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

// retain the publication palette within the shared semantic roles
Object.assign(theme.style, {
    "--destack-color-background": "light-dark(#f8f7f4, #1b1a19)",
    "--destack-color-foreground": "light-dark(#272624, #d5d2cb)",
    "--destack-color-muted": "light-dark(#efeeea, #292725)",
    "--destack-color-mutedForeground": "light-dark(#68655f, #a39e94)",
    "--destack-color-border": "light-dark(#dedbd5, #3d3a36)",
    "--destack-color-primary": "light-dark(#b95532, #d68b65)",
});

/** Render the shared HTML document. */
export default function Document(props: ParentProps) {
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
                <div id="app">{props.children}</div>
            </body>
        </html>
    );
}

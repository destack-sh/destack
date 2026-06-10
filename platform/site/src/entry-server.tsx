import { createHandler, StartServer } from "@solidjs/start/server";
import type { JSX } from "solid-js";

export default createHandler(() => <StartServer document={Document} />);

// resolve the theme before first paint: stored choice first, system preference otherwise
const themeScript = `(() => {
    const override = new URLSearchParams(location.search).get("theme");
    const stored = override ?? localStorage.getItem("destack-theme");
    const system = matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
    document.documentElement.dataset.theme = stored === "dark" || stored === "light" ? stored : system;
})();`;

function Document(props: { assets: JSX.Element; children?: JSX.Element; scripts: JSX.Element }) {
    return (
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <link rel="icon" href="/brand/favicon/favicon.svg" type="image/svg+xml" />
                <script innerHTML={themeScript} />
                {props.assets}
            </head>
            <body>
                <div id="app">{props.children}</div>
                {props.scripts}
            </body>
        </html>
    );
}

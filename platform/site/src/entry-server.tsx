import { createHandler, StartServer } from "@solidjs/start/server";
import type { JSX } from "solid-js";

export default createHandler(() => <StartServer document={Document} />);

/// The server-rendered document fields supplied by SolidStart.
type DocumentProps = {
    /// The styles and metadata collected during rendering.
    assets: JSX.Element;

    /// The rendered application.
    children?: JSX.Element;

    /// The client runtime scripts.
    scripts: JSX.Element;
};

/// Render the server document.
function Document(props: DocumentProps) {
    return (
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <link rel="icon" href="/brand/favicon/favicon.svg" type="image/svg+xml" />
                {props.assets}
            </head>
            <body>
                <div id="app">{props.children}</div>
                {props.scripts}
            </body>
        </html>
    );
}

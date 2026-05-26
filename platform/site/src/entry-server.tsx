import { createHandler, StartServer } from "@solidjs/start/server";
import type { JSX } from "solid-js";

export default createHandler(() => <StartServer document={Document} />);

function Document(props: { assets: JSX.Element; children: JSX.Element; scripts: JSX.Element }) {
    return (
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <title>Destack</title>
                <meta
                    name="description"
                    content="Destack is a universal software engine for building correct, optimal, integrated software."
                />
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

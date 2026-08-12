import ibmPlexMono from "@fontsource/ibm-plex-mono/files/ibm-plex-mono-latin-400-normal.woff2?url";
import { Link, MetaProvider } from "@solidjs/meta";
import { Router } from "@solidjs/router";
import { FileRoutes } from "@solidjs/start/router";
import * as stylex from "@stylexjs/stylex";
import { Suspense } from "solid-js";

import { styles } from "./style/app.stylex";
import "./style/site.css";

/// Render the site router and shared document providers.
export default function App() {
    return (
        <Router
            root={(props) => (
                <MetaProvider>
                    <Link
                        as="font"
                        crossorigin="anonymous"
                        href={ibmPlexMono}
                        rel="preload"
                        type="font/woff2"
                    />
                    <Suspense>
                        <div {...stylex.attrs(styles.root)}>
                            {props.children}
                        </div>
                    </Suspense>
                </MetaProvider>
            )}
        >
            <FileRoutes />
        </Router>
    );
}

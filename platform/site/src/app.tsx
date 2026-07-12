import jetbrainsMono from "@fontsource-variable/jetbrains-mono/files/jetbrains-mono-latin-wght-normal.woff2?url";
import { Link, MetaProvider } from "@solidjs/meta";
import { Router } from "@solidjs/router";
import { FileRoutes } from "@solidjs/start/router";
import { Suspense } from "solid-js";

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
                        href={jetbrainsMono}
                        rel="preload"
                        type="font/woff2"
                    />
                    <Suspense>
                        <div class="min-h-screen bg-destack-page text-destack-text selection:bg-destack-accent selection:text-neutral-950">
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

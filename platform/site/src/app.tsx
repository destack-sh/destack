import { MetaProvider } from "@solidjs/meta";
import { Router } from "@solidjs/router";
import { FileRoutes } from "@solidjs/start/router";
import { Suspense } from "solid-js";

import "./style.css";

export default function App() {
    return (
        <Router
            root={(props) => (
                <MetaProvider>
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

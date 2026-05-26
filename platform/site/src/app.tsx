import { Router } from "@solidjs/router";
import { FileRoutes } from "@solidjs/start/router";
import { Suspense } from "solid-js";

import "./style.css";

export default function App() {
    return (
        <Router
            root={(props) => (
                <Suspense>
                    <div class="min-h-screen bg-destack-page text-neutral-950 selection:bg-destack-accent selection:text-neutral-950">
                        {props.children}
                    </div>
                </Suspense>
            )}
        >
            <FileRoutes />
        </Router>
    );
}

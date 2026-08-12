import { Navigate } from "@solidjs/router";

/// Forward the manual root to its first chapter.
export default function Documentation() {
    return <Navigate href="/docs/overview/" />;
}

import { Navigate } from "@solidjs/router";

/// Forward `/docs` to the language reference.
export default function Reference() {
    return <Navigate href="/docs/language/" />;
}

import { Navigate } from "@solidjs/router";

/// Forward `/docs` to the language guide.
export default function Guide() {
    return <Navigate href="/docs/language/" />;
}

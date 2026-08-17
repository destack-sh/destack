import { useParams } from "@solidjs/router";
import { Show } from "solid-js";

import { documentByRoute } from "../../generated/documents";
import { DocumentPage } from "../../page/document";
import { LibraryItemPage } from "../../page/reference";

export default function DocumentationPage() {
    const params = useParams();
    const path = () => params.path?.replace(/^\/+|\/+$/g, "") ?? "";
    const route = () => `/docs/${path()}/`;
    const isLibraryItem = () => (
        route().startsWith("/docs/language/library/") && !documentByRoute.has(route())
    );

    return (
        <Show
            fallback={<DocumentPage route={route()} />}
            when={isLibraryItem()}
        >
            <LibraryItemPage route={route()} />
        </Show>
    );
}

import { useParams } from "@solidjs/router";
import { Show } from "solid-js";

import { isGeneratedReferenceRoute } from "../../content/reference";
import { documentByRoute } from "../../generated/documents";
import { DocumentPage } from "../../page/document";
import { GeneratedReferencePage } from "../../page/reference";

export default function DocumentationPage() {
    const params = useParams();
    const path = () => params.path?.replace(/^\/+|\/+$/g, "") ?? "";
    const route = () => `/docs/${path()}/`;
    const isGeneratedReference = () => (
        isGeneratedReferenceRoute(route()) &&
        !documentByRoute.has(route())
    );

    return (
        <Show
            fallback={<DocumentPage route={route()} />}
            when={isGeneratedReference()}
        >
            <GeneratedReferencePage route={route()} />
        </Show>
    );
}

import { useParams } from "@solidjs/router";

import { DocumentPage } from "../../page/document";

export default function DocumentationPage() {
    const params = useParams();
    const path = () => params.path?.replace(/^\/+|\/+$/g, "") ?? "";

    return <DocumentPage route={`/docs/${path()}/`} />;
}

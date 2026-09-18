import { useParams } from "@destack/view/router";
import { DocumentPage } from "../../page/document";

/// Render a canonical documentation route.
export default function DocumentationPage() {
    const params = useParams();
    const route = () => `/docs/${params.path?.replace(/^\/+|\/+$/g, "") ?? ""}/`;

    return <DocumentPage route={route()} />;
}

import { useParams } from "@destack/view/router";
import { DocumentPage } from "../../page/document";

/** Render a canonical documentation route. */
export default function DocumentationPage() {
    const parameters = useParams();
    const route = () => `/docs/${parameters.path?.replace(/^\/+|\/+$/g, "") ?? ""}/`;

    return <DocumentPage route={route()} />;
}

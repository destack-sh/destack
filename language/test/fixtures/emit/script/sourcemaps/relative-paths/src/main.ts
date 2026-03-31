import { renderDocsPageRoute } from "./routes/docs/page.ts";
import { renderDocsCard } from "./routes/docs/render/card.ts";
import { currentSlug } from "./state/docs.ts";

const docsPageRoute = renderDocsPageRoute(currentSlug);
const docsCard = renderDocsCard("emit", docsPageRoute);

export default docsCard;

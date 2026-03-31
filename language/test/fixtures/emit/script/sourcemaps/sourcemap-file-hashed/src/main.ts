import { renderGuideSummary } from "./routes/guides/render.ts";
import { renderGuideCard } from "./routes/guides/card.ts";
import { currentGuide } from "./routes/guides/state.ts";

const guideSummary = renderGuideSummary(
    currentGuide.slug,
    currentGuide.section,
);
const guideCard = renderGuideCard("guides", guideSummary);

console.log(guideCard);

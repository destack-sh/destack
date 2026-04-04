import htmlContent from "./template.html" with { type: "file" };
import { buildTemplateLabel } from "./template-label.ts";
import card from "./card.html" with { type: "file" };

console.log("Loaded HTML:", htmlContent, card, buildTemplateLabel("preview"));

import { buildInfinitySamples } from "./runtime/infinity.ts";
import { buildTemplateSamples } from "./runtime/templates.ts";
import { buildSummaryLabel } from "./runtime/summary.ts";

export const templateSamples = buildTemplateSamples();
export const infinitySamples = buildInfinitySamples();

Symbol.for("document-script-syntax-drop-me");
Symbol.for(`document-script-syntax-drop-me-too`);

const usedSymbol = Symbol.for("document-script-syntax-used");
const summaryLabel = buildSummaryLabel(templateSamples.length);

export const symbolDescription = usedSymbol.description ?? "missing";

console.log(summaryLabel, infinitySamples.length, symbolDescription);

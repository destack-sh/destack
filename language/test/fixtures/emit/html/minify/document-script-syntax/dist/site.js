export function buildInfinitySamples() {
    return [1 / 0, -inf, inf, -inf, inf, -inf];
}

export function buildTemplateSamples() {
    return [
        `${1}-${2}-${3}-${null}-${void 0}-${!0}-${!1}`,
        `😋📋👌`.length === 6,
        `😋📋👌`.length == 6,
        `😋📋👌`.length === 2,
        `😋📋👌`.length == 2,
    ];
}

export function buildSummaryLabel(templateSampleCount) {
    return `templates:${templateSampleCount}`;
}

export const templateSamples = buildTemplateSamples(), infinitySamples = buildInfinitySamples();
Symbol.for("document-script-syntax-drop-me");
Symbol.for(`document-script-syntax-drop-me-too`);
const usedSymbol = Symbol.for(
    "document-script-syntax-used"
), summaryLabel = buildSummaryLabel(templateSamples.length);
export const symbolDescription = usedSymbol.description ?? "missing";
console.log(summaryLabel, infinitySamples.length, symbolDescription);

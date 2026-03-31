export function renderSourceMapCard(section, label) {
    return `${section}:${label}`;
}

export function buildSourceMapLabel(section) {
    return `source-map:${section}`;
}

export function runExternalSourceMapped(fn) {
    return fn();
}

runExternalSourceMapped(function() {
    const sourceMapLabel = buildSourceMapLabel("input");
    return renderSourceMapCard("input", sourceMapLabel);
});
//# sourceMappingURL=./site.js.map

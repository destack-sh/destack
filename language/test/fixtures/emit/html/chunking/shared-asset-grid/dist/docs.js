export function getSharedSummary() {
    return "shared-grid";
}

export function renderPage(name) {
    return `${name}:${getSharedSummary()}`;
}

console.log(renderPage("docs"));
//# sourceMappingURL=./docs.js.map

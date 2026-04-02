export function getSharedSummary() {
    return "shared-grid";
}

export function renderPage(name) {
    return `${name}:${getSharedSummary()}`;
}

console.log(renderPage("pricing"));
//# sourceMappingURL=./pricing.js.map

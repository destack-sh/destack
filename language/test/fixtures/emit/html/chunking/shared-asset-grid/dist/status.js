export function getSharedSummary() {
    return "shared-grid";
}

export function renderPage(name) {
    return `${name}:${getSharedSummary()}`;
}

console.log(renderPage("status"));
//# sourceMappingURL=./status.js.map

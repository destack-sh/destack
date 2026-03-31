export function escapePreviewTitle(title: string) {
    return title.replaceAll("&", "&amp;").replaceAll("\"", "&quot;");
}

export function renderPreviewSummary(name: string, route: string) {
    return `${name}:${route}`;
}

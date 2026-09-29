/** Escape text inserted into generated HTML. */
export function escapeHtml(value: string): string {
    return value
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll('"', "&quot;");
}

/** Escape text inserted into an HTML attribute. */
export function escapeAttribute(value: string): string {
    return escapeHtml(value).replaceAll("'", "&#39;");
}

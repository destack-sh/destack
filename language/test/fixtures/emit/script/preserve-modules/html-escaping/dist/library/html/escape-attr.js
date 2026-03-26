import { escapeHtml } from "./escape-html.js";
export function escapeAttr(value) {
    return escapeHtml(value).replace(/'/g, "&#39;");
}
export function quoteAttr(name, value) {
    const escapedValue = escapeAttr(value);
    return `${name}="${escapedValue}"`;
}
//# sourceMappingURL=./escape-attr.map

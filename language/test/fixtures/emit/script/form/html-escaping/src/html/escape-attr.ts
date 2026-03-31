import { escapeHtml } from "./escape-html.ts";

/** Escape one html attribute value. */
export function escapeAttr(value: string): string {
    return escapeHtml(value).replace(/'/g, "&#39;");
}

/** Build one quoted html attribute. */
export function quoteAttr(name: string, value: string): string {
    const escapedValue = escapeAttr(value);

    return `${name}="${escapedValue}"`;
}

export const previewBaseUrl = import.meta.url;
export const previewMeta = import.meta;

export function buildPreviewHref(name: string) {
    return `${name}.html`;
}

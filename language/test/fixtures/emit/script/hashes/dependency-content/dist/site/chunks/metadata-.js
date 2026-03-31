export const sharedMetadata = { channel: "shared", kind: "page-summary" };

export function renderSharedSummary(channel, headline) {
    return `${channel}:${headline}`;
}

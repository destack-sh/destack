/** A browser's share sheet, as the Web Share API has it. */
export interface Share {
    /** Report whether the browser can share some data. */
    readonly canShare: (data: ShareData) => boolean;
    /** Share data through the browser's share sheet. */
    readonly share: (data: ShareData) => Promise<void>;
}

/** Shares. */
export const Share: { readonly system: Share } = {
    /** The browser's share sheet. */
    system: {
        canShare: (data) => navigator.canShare(data),
        share: (data) => navigator.share(data),
    },
};

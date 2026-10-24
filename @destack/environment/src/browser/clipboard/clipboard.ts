/** A browser's clipboard, as the Async Clipboard API has it. */
export interface Clipboard {
    /** Read the clipboard's items. */
    readonly read: () => Promise<ClipboardItem[]>;
    /** Write items to the clipboard. */
    readonly write: (items: ClipboardItem[]) => Promise<void>;
    /** Write text to the clipboard. */
    readonly writeText: (text: string) => Promise<void>;
}

/** Clipboards. */
export const Clipboard: { readonly system: Clipboard } = {
    /** The browser's clipboard. */
    system: {
        read: () => navigator.clipboard.read(),
        write: (items) => navigator.clipboard.write(items),
        writeText: (text) => navigator.clipboard.writeText(text),
    },
};

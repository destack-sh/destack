/** The RFC 5321 envelope of one message: its sender and every recipient, blind copies included. */
export interface Envelope {
    /** The reverse-path address, which bounces return to. */
    readonly sender: string;
    /** The forward-path addresses, in order. */
    readonly recipients: readonly string[];
}

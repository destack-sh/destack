/** The prefix of every label, the labelling scheme's version, as MLS prefixes "MLS 1.0 ". */
const LABEL_PREFIX = "destack 1.0 ";

/** The longest prefixed label an encoding holds, its length field one byte (RFC 9420). */
const MAX_LABEL_BYTES = 255;

/** The longest context an encoding holds, its length field two bytes. */
const MAX_CONTEXT_BYTES = 65_535;

/** A key's purpose and what it binds to, as MLS labels its derivations and encryptions. */
export interface Label {
    /** The purpose, namespaced by the package owning the key, such as `@destack/db/column`. */
    readonly label: string;
    /** What the key or ciphertext is bound to, such as an installation, empty when nothing. */
    readonly context: string;
}

/** The encodings of labels. */
export const Label = {
    /** Encode a label as an encryption context: the prefixed label and the context, each sized. */
    encode(label: Label): Uint8Array<ArrayBuffer> {
        // refuse a label or a context its length field cannot hold
        const encoder = new TextEncoder();
        const prefixed = encoder.encode(`${LABEL_PREFIX}${label.label}`);
        const context = encoder.encode(label.context);
        if (prefixed.length > MAX_LABEL_BYTES || context.length > MAX_CONTEXT_BYTES) {
            throw new TypeError(`label ${label.label} or its context is too long`);
        }

        // write each field after its length, uint8 for the label and uint16 for the context
        const encoded = new Uint8Array(1 + prefixed.length + 2 + context.length);
        const view = new DataView(encoded.buffer);
        view.setUint8(0, prefixed.length);
        encoded.set(prefixed, 1);
        view.setUint16(1 + prefixed.length, context.length);
        encoded.set(context, 3 + prefixed.length);

        return encoded;
    },

    /** Encode a label as a derivation's info: the output length before the encryption context. */
    encodeExpansion(label: Label, bytes: number): Uint8Array<ArrayBuffer> {
        // write the output length as two bytes before the encoded label
        const encoded = Label.encode(label);
        const info = new Uint8Array(2 + encoded.length);
        new DataView(info.buffer).setUint16(0, bytes);
        info.set(encoded, 2);

        return info;
    },
};

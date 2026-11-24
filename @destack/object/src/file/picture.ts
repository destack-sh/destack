import { schema } from "@destack/schema";

/** The size of a picture in pixels, as it is shown upright. */
export const PictureSize = schema.object({
    /** The width. */
    width: schema.number().int().positive(),
    /** The height. */
    height: schema.number().int().positive(),
});
/** The size of a picture in pixels, as it is shown upright. */
export type PictureSize = schema.Infer<typeof PictureSize>;

/** The bytes a picture's size is read from, past the metadata cameras write before it: 256 KiB. */
export const PICTURE_HEADER_BYTES = 256 * 1024;

/** The first bytes of a PNG file (PNG 5.2), then its header chunk. */
const PNG = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

/** The markers of a JPEG frame's start, which has its size (ITU T.81 B.2.2). */
const JPEG_FRAMES: ReadonlySet<number> = new Set([
    0xc0, 0xc1, 0xc2, 0xc3, 0xc5, 0xc6, 0xc7, 0xc9, 0xca, 0xcb, 0xcd, 0xce, 0xcf,
]);

/** The marker that starts a JPEG (ITU T.81 B.1.1.3). */
const JPEG_START = 0xffd8;

/** The byte before each JPEG marker's own (ITU T.81 B.1.1.2). */
const JPEG_MARKER = 0xff;

/** The marker of a JPEG segment with Exif metadata (Exif 2.32, 4.7.2). */
const EXIF_MARKER = 0xe1;

/** The first bytes of Exif metadata, "Exif" (Exif 2.32, 4.7.2). */
const EXIF = 0x45786966;

/** The byte order mark of Exif metadata with the lowest byte first, "II" (TIFF 6.0). */
const LITTLE_ENDIAN = 0x4949;

/** The bytes of an entry of Exif metadata (TIFF 6.0). */
const ENTRY_BYTES = 12;

/** The tag of a picture's orientation (Exif 2.32, 4.6.4). */
const ORIENTATION_TAG = 0x0112;

/** The first orientation that turns a picture on its side. */
const TURNED = 5;

/** Read a picture's size from its first bytes, absent for another file or an unknown format. */
export function pictureSize(bytes: Uint8Array<ArrayBuffer>): PictureSize | undefined {
    // tell the format by its first bytes, refusing sizes no picture has
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    let size: PictureSize | undefined;
    try {
        if (PNG.every((byte, place) => bytes[place] === byte)) {
            size = { width: view.getUint32(16), height: view.getUint32(20) };
        } else if (view.getUint16(0) === JPEG_START) {
            size = jpegSize(view);
        } else if (textAt(bytes, 0, 4) === "GIF8") {
            size = { width: view.getUint16(6, true), height: view.getUint16(8, true) };
        } else if (textAt(bytes, 0, 4) === "RIFF" && textAt(bytes, 8, 12) === "WEBP") {
            size = webpSize(bytes, view);
        }
    } catch (error) {
        // take a header that ends early as no picture
        if (!(error instanceof RangeError)) {
            throw error;
        }
    }

    return size !== undefined && PictureSize.safeParse(size).success ? size : undefined;
}

/** Read a JPEG's size from its frame, turned as its camera was. */
function jpegSize(view: DataView): PictureSize | undefined {
    let isTurned = false;
    for (let at = 2; at < view.byteLength;) {
        // read the next segment, the camera's orientation or the frame
        const marker = view.getUint8(at + 1);
        const length = view.getUint16(at + 2);
        if (view.getUint8(at) !== JPEG_MARKER) {
            return undefined;
        } else if (JPEG_FRAMES.has(marker)) {
            const [height, width] = [view.getUint16(at + 5), view.getUint16(at + 7)];

            return isTurned ? { width: height, height: width } : { width, height };
        } else if (marker === EXIF_MARKER && view.getUint32(at + 4) === EXIF) {
            isTurned ||= orientationAt(view, at + 10) >= TURNED;
        }
        at += 2 + length;
    }

    return undefined;
}

/** Read the orientation of the Exif metadata at an offset, upright where it has none. */
function orientationAt(view: DataView, start: number): number {
    // read the byte order, then the entries of the first directory
    const isLittle = view.getUint16(start) === LITTLE_ENDIAN;
    const directory = start + view.getUint32(start + 4, isLittle);
    const count = view.getUint16(directory, isLittle);
    for (let entry = 0; entry < count; entry += 1) {
        const at = directory + 2 + entry * ENTRY_BYTES;
        if (view.getUint16(at, isLittle) === ORIENTATION_TAG) {
            return view.getUint16(at + 8, isLittle);
        }
    }

    return 1;
}

/** Read a WebP's size from its first chunk: lossy, lossless or extended (RFC 9649). */
function webpSize(bytes: Uint8Array, view: DataView): PictureSize | undefined {
    const chunk = textAt(bytes, 12, 16);
    if (chunk === "VP8 ") {
        return {
            width: view.getUint16(26, true) & 0x3fff,
            height: view.getUint16(28, true) & 0x3fff,
        };
    } else if (chunk === "VP8L") {
        const packed = view.getUint32(21, true);

        return { width: (packed & 0x3fff) + 1, height: ((packed >>> 14) & 0x3fff) + 1 };
    } else if (chunk === "VP8X") {
        const read = (at: number): number =>
            view.getUint16(at, true) + (view.getUint8(at + 2) << 16) + 1;

        return { width: read(24), height: read(27) };
    }

    return undefined;
}

/** Read some bytes as ASCII text. */
function textAt(bytes: Uint8Array, from: number, to: number): string {
    return String.fromCharCode(...bytes.subarray(from, to));
}

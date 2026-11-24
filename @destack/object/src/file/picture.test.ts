import { expect, test } from "@destack/test";
import { pictureSize } from "./index.ts";

/** Write the bytes of hexadecimal groups. */
function bytesOf(...groups: readonly string[]): Uint8Array<ArrayBuffer> {
    return Uint8Array.fromHex(groups.join("").replaceAll(" ", ""));
}

/** A JPEG's start, with a camera's metadata of an orientation. */
function exif(orientation: string): string {
    const directory = `0001 0112 0003 00000001 ${orientation}0000 00000000`;

    return `ffe1 0022 457869660000 4d4d002a00000008 ${directory}`;
}

test("read the size of pictures from their first bytes, upright, and of nothing else", () => {
    const frame = "ffc0 0011 08 0190 0258 03 012200 021101 031101";

    expect({
        png: pictureSize(bytesOf("89504e470d0a1a0a 0000000d 49484452 00000280 000001e0 0802")),
        jpeg: pictureSize(bytesOf("ffd8 ffe0 0010 4a46494600010100000100010000", frame)),
        turned: pictureSize(bytesOf("ffd8", exif("0006"), frame)),
        upright: pictureSize(bytesOf("ffd8", exif("0001"), frame)),
        gif: pictureSize(bytesOf("474946383961 2003 5802 f70000")),
        lossy: pictureSize(
            bytesOf("52494646 00000000 57454250 56503820 00000000 000000 9d012a 8002 e001"),
        ),
        lossless: pictureSize(
            bytesOf("52494646 00000000 57454250 5650384c 00000000 2f 7fc27700 00"),
        ),
        extended: pictureSize(
            bytesOf("52494646 00000000 57454250 56503858 0a000000 10000000 7f0200 df0100"),
        ),
        cut: pictureSize(bytesOf("ffd8 ffe0")),
        unmarked: pictureSize(bytesOf("ffd8 00c0 0011 08 0190 0258 03")),
        empty: pictureSize(bytesOf("89504e470d0a1a0a 0000000d 49484452 00000000 000001e0")),
        text: pictureSize(new TextEncoder().encode("a text")),
        none: pictureSize(new Uint8Array()),
    }).toEqual({
        png: { width: 640, height: 480 },
        jpeg: { width: 600, height: 400 },
        turned: { width: 400, height: 600 },
        upright: { width: 600, height: 400 },
        gif: { width: 800, height: 600 },
        lossy: { width: 640, height: 480 },
        lossless: { width: 640, height: 480 },
        extended: { width: 640, height: 480 },
        cut: undefined,
        unmarked: undefined,
        empty: undefined,
        text: undefined,
        none: undefined,
    });
});

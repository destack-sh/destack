import { createHash } from "node:crypto";
import { expect, test } from "@destack/test";
import { md5 } from "./md5.ts";

test("digest RFC 1321's test suite and agree with node:crypto across block boundaries", () => {
    // the RFC's own vectors
    const vectors = [
        text(""),
        text("a"),
        text("abc"),
        text("message digest"),
        text("abcdefghijklmnopqrstuvwxyz"),
    ];

    // every length around one and two blocks, against node:crypto
    const lengths = Array.from({ length: 140 }, (_, index) => index);
    const disagreeing = lengths.filter((length) => {
        const bytes = Uint8Array.from({ length }, (_, index) => (index * 31 + 7) % 256);

        return md5(bytes).toHex() !== createHash("md5").update(bytes).digest("hex");
    });

    expect([vectors, disagreeing]).toEqual([
        [
            "d41d8cd98f00b204e9800998ecf8427e",
            "0cc175b9c0f1b6a831c399e269772661",
            "900150983cd24fb0d6963f7d28e17f72",
            "f96b697d7cb7938d525a2f31aaf161d0",
            "c3fcd3d76192e4007dfb496cca67e13b",
        ],
        [],
    ]);
});

/** Digest a text's UTF-8 bytes as hexadecimal. */
function text(value: string): string {
    return md5(new TextEncoder().encode(value)).toHex();
}

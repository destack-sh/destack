import { Digest, nth } from "@destack/schema";
import { ResourceError } from "../error/error.ts";
import type { BlobBody, BlobPlacement } from "./provider.ts";

/** The prefix of a pack of several blobs, a key no digest takes. */
const PACK_PREFIX = "pack-";

/** The size from which a blob gets a pack of its own: 1 MiB, copied in under 1 ms below it. */
const LONE_BYTES = 1024 * 1024;

/** A pack's key of fresh name: the prefix, then a UUID. */
const PACK_KEY = new RegExp(`^${PACK_PREFIX}[0-9a-f-]+$`, "u");

/** Packs of blobs: their bytes back to back under one key, as git packs objects. */
export const BlobPack = {
    /** Group bodies in order into packs: each large body alone, each run of small ones together. */
    groups<Body extends Pick<BlobBody, "size">>(bodies: readonly Body[]): Body[][] {
        const groups: Body[][] = [];
        for (const body of bodies) {
            // join a small body to a run of small ones, else start a pack
            const last = groups.at(-1);
            const isJoining =
                !BlobPack.isLone(body) && last !== undefined && !BlobPack.isLone(nth(last, 0));
            if (isJoining) {
                last.push(body);
            } else {
                groups.push([body]);
            }
        }

        return groups;
    },

    /** Report whether a body is large enough for a pack of its own. */
    isLone(body: Pick<BlobBody, "size">): boolean {
        return (body.size ?? 0) >= LONE_BYTES;
    },

    /** Key the pack of some blobs: a lone blob's digest where known, else a fresh pack name. */
    key(blobs: readonly Pick<BlobBody, "digest">[], fresh: () => string): string {
        const [only] = blobs;

        return blobs.length === 1 && only?.digest !== undefined
            ? only.digest
            : `${PACK_PREFIX}${fresh()}`;
    },

    /** Report whether a name is a pack's key: a digest, or a pack of fresh name. */
    isKey(name: string): boolean {
        return Digest.safeParse(name).success || PACK_KEY.test(name);
    },

    /** Refuse a body's bytes unlike its expected digest or size. */
    verify(body: Pick<BlobBody, "digest" | "size">, digest: Digest, size: number): void {
        // refuse another digest
        if (body.digest !== undefined && digest !== body.digest) {
            throw ResourceError.of(
                "DIGEST_INVALID",
                `blob '${digest}' is invalid: its digest must be '${body.digest}'`,
            );
        }
        // refuse another size
        else if (body.size !== undefined && size !== body.size) {
            throw ResourceError.of(
                "DIGEST_INVALID",
                `blob '${digest}' is invalid: it has ${size} bytes and must have ${body.size} bytes`,
            );
        }
    },

    /** Place blobs one after another in a pack. */
    place(
        pack: string,
        blobs: readonly { readonly digest: Digest; readonly size: number }[],
    ): BlobPlacement[] {
        let offset = 0;

        return blobs.map(({ digest, size }) => {
            const placement = { digest, pack, offset, size };
            offset += size;

            return placement;
        });
    },
};

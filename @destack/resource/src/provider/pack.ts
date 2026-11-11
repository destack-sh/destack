import { Digest } from "@destack/schema";
import { ResourceError } from "../error/error.ts";
import type { BlobBody, BlobPlacement } from "./provider.ts";

/** The prefix of a pack of several blobs, a key no digest takes. */
const PACK_PREFIX = "pack-";

/** A pack's key of fresh name: the prefix, then a UUID. */
const PACK_KEY = new RegExp(`^${PACK_PREFIX}[0-9a-f-]+$`, "u");

/** Packs of blobs: their bytes back to back under one key, as git packs objects. */
export const BlobPack = {
    /** Key the pack of some bodies: a lone body's expected digest, else a fresh pack name. */
    key(bodies: readonly BlobBody[], fresh: () => string): string {
        const [only] = bodies;

        return bodies.length === 1 && only?.digest !== undefined
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

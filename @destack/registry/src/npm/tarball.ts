import { Tarball, TarballEntry } from "@destack/package/archive";
import type { Crypto } from "@destack/environment";
import type { BuildReader } from "@destack/package/manifest";
import { schema, Text } from "@destack/schema";
import { RegistryError } from "../error/index.ts";

/** The directory npm clients expect every file of a tarball below. */
const ROOT = "package";

/** The file npm clients read a release's name, version and dependencies from. */
const PACKAGE_FILE = "package.json";

/** The fields of a package file listing dependencies by range. */
const DEPENDENCY_FIELDS = [
    "dependencies",
    "devDependencies",
    "peerDependencies",
    "optionalDependencies",
] as const;

/** The start of a range selecting a package of the same workspace, as Bun and pnpm write it. */
const WORKSPACE_RANGE = "workspace:";

/** A release as npm clients install it: its build's files below package/, at released versions. */
export const NpmTarball = {
    /** The path of the package file inside the archive. */
    packageFile: `${ROOT}/${PACKAGE_FILE}`,

    /** List a build's files as entries in path order, its package file at the linked versions. */
    async entries(
        reader: BuildReader,
        versions: Readonly<Record<string, string>>,
    ): Promise<TarballEntry[]> {
        // take the files in path order, so equal builds give equal bytes
        const files = (await reader.files()).toSorted((left, right) =>
            Text.compare(left.path, right.path),
        );

        return Promise.all(
            files.map(async (file) => {
                // release the package file, and take every other file as the build has it
                const bytes = await reader.load(file.path);
                const contents =
                    file.path === PACKAGE_FILE ? NpmTarball.release(bytes, versions) : bytes;

                return TarballEntry.of(`${ROOT}/${file.path}`, contents);
            }),
        );
    },

    /** Write a package file with each workspace range replaced by the linked release's version. */
    release(
        authored: Uint8Array,
        versions: Readonly<Record<string, string>>,
    ): Uint8Array<ArrayBuffer> {
        // replace the workspace ranges of each dependency field it has
        const manifest = schema.jsonObject().parse(JSON.parse(new TextDecoder().decode(authored)));
        const released: Record<string, unknown> = { ...manifest };
        for (const field of DEPENDENCY_FIELDS) {
            const ranges = schema
                .record(schema.string(), schema.string())
                .safeParse(manifest[field]);
            if (ranges.success) {
                released[field] = Object.fromEntries(
                    Object.entries(ranges.data).map(([name, range]) => [
                        name,
                        range.startsWith(WORKSPACE_RANGE)
                            ? NpmTarball.linked(name, versions)
                            : range,
                    ]),
                );
            }
        }

        return new TextEncoder().encode(`${JSON.stringify(released, null, 4)}\n`);
    },

    /** Read the version a workspace dependency's release has, refusing one the lock lacks. */
    linked(name: string, versions: Readonly<Record<string, string>>): string {
        const version = versions[name];
        if (version === undefined) {
            throw RegistryError.of(
                "MANIFEST_INVALID",
                `dependency '${name}' is invalid: its release is not linked`,
            );
        }

        return version;
    },

    /** Archive some entries as a gzip file of stored blocks, equal entries giving equal bytes. */
    async archive(entries: readonly TarballEntry[]): Promise<Uint8Array<ArrayBuffer>> {
        // collect the tar archive's bytes
        const chunks: Uint8Array[] = [];
        const archived = Tarball.stream(
            (async function* () {
                yield* entries;
            })(),
        );
        for await (const chunk of archived) {
            chunks.push(chunk);
        }
        const size = chunks.reduce((sum, chunk) => sum + chunk.byteLength, 0);
        const tar = new Uint8Array(size);
        let offset = 0;
        for (const chunk of chunks) {
            tar.set(chunk, offset);
            offset += chunk.byteLength;
        }

        return Gzip.store(tar);
    },

    /** Hash an archive as npm's subresource integrity: `sha512-` and the base64 digest. */
    async integrity(
        archive: Uint8Array<ArrayBuffer>,
        crypto: Pick<Crypto, "digest">,
    ): Promise<string> {
        const digest = new Uint8Array(await crypto.digest("SHA-512", archive));

        return `sha512-${digest.toBase64()}`;
    },
};

/** The gzip header: deflate, no flags, no time, no extra flags, an unknown system (RFC 1952). */
const GZIP_HEADER = [0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff];

/** The most bytes one stored deflate block holds: its length is 16 bits (RFC 1951). */
const STORED_BLOCK_BYTES = 0xffff;

/** The bytes of a stored block's header: its final flag, its length and the complement. */
const STORED_BLOCK_OVERHEAD = 5;

/** The bytes after the blocks: the CRC-32 and the size, each 32 bits. */
const GZIP_TRAILER_BYTES = 8;

/** The reflected polynomial of CRC-32 (ISO 3309), as gzip uses it. */
const CRC_POLYNOMIAL = 0xedb88320;

/** The CRC-32 of each byte value. */
const CRC_TABLE = Uint32Array.from({ length: 256 }, (_, value) => {
    let remainder = value;
    for (let bit = 0; bit < 8; bit += 1) {
        remainder = remainder & 1 ? (remainder >>> 1) ^ CRC_POLYNOMIAL : remainder >>> 1;
    }

    return remainder >>> 0;
});

/** Gzip files whose bytes depend on their input alone, as no compressor chooses them. */
const Gzip = {
    /** Wrap bytes in a gzip file of stored blocks. */
    store(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
        // size the file: the header, each block with its overhead, and the trailer
        const blocks = Math.max(1, Math.ceil(bytes.byteLength / STORED_BLOCK_BYTES));
        const size =
            GZIP_HEADER.length +
            blocks * STORED_BLOCK_OVERHEAD +
            bytes.byteLength +
            GZIP_TRAILER_BYTES;
        const file = new Uint8Array(size);
        const view = new DataView(file.buffer);
        file.set(GZIP_HEADER, 0);

        // write each block: its final flag, its length, the length's complement, then its bytes
        let offset = GZIP_HEADER.length;
        for (let block = 0; block < blocks; block += 1) {
            const start = block * STORED_BLOCK_BYTES;
            const data = bytes.subarray(start, start + STORED_BLOCK_BYTES);
            file[offset] = block === blocks - 1 ? 1 : 0;
            view.setUint16(offset + 1, data.byteLength, true);
            view.setUint16(offset + 3, data.byteLength ^ 0xffff, true);
            file.set(data, offset + STORED_BLOCK_OVERHEAD);
            offset += STORED_BLOCK_OVERHEAD + data.byteLength;
        }

        // end with the checksum and the size of the bytes
        view.setUint32(offset, Gzip.checksum(bytes), true);
        view.setUint32(offset + 4, bytes.byteLength >>> 0, true);

        return file;
    },

    /** Compute the CRC-32 of bytes. */
    checksum(bytes: Uint8Array): number {
        let remainder = 0xffffffff;
        for (const byte of bytes) {
            remainder = (CRC_TABLE[(remainder ^ byte) & 0xff] ?? 0) ^ (remainder >>> 8);
        }

        return (remainder ^ 0xffffffff) >>> 0;
    },
};

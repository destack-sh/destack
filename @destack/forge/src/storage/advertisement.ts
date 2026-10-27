import { ForgeError } from "../error/index.ts";
import type { Lease } from "@destack/resource";
import type { Fetch } from "@destack/object";
import type { GitListing, GitReference } from "./storage.ts";

/** The content type of a smart HTTP upload-pack reference advertisement. */
const ADVERTISEMENT_TYPE = "application/x-git-upload-pack-advertisement";

/** The width of a pkt-line's hexadecimal length prefix, included in the length. */
const LENGTH_WIDTH = 4;

/** The suffix of an advertised line with the commit an annotated tag peels to. */
const PEELED_SUFFIX = "^{}";

/** The capability with the branch HEAD points to. */
const HEAD_CAPABILITY = "symref=HEAD:";

/** The branches and tags a Git server advertises over the smart HTTP protocol. */
export const GitAdvertisement = {
    /** Read a remote's advertisement from GET <remote>/info/refs?service=git-upload-pack. */
    async read(lease: Lease, fetch: Fetch): Promise<GitListing> {
        // request the upload-pack advertisement as Git does, with the lease's headers
        const headers = new Headers({ ...lease.headers, "User-Agent": "git/destack" });
        const url = `${lease.url.replace(/\/$/u, "")}/info/refs?service=git-upload-pack`;
        const response = await fetch(new Request(url, { headers }));

        // require a smart HTTP advertisement
        const type = response.headers.get("content-type");
        if (!response.ok || type !== ADVERTISEMENT_TYPE) {
            throw ForgeError.of(
                "REMOTE_FAILED",
                `git remote responded with ${response.status} ${type ?? "without a content type"}`,
            );
        }

        return GitAdvertisement.parse(new Uint8Array(await response.arrayBuffer()));
    },

    /** Parse an advertisement's pkt-lines into its branches, tags and default branch. */
    parse(body: Uint8Array): GitListing {
        // read each pkt-line after the service announcement, skipping flush packets
        const lines = GitAdvertisement.packetLines(body);
        if (lines[0] !== "# service=git-upload-pack") {
            throw ForgeError.of("UPLOAD_PACK_MISSING", "git remote did not announce upload-pack");
        }

        // read the capabilities on the first reference line, and each reference and its peeled commit
        let head: string | null = null;
        const objects = new Map<string, string>();
        const peeled = new Map<string, string>();
        for (const line of lines.slice(1)) {
            const [advertised, capabilities] = line.split("\0");
            const [object, name] = advertised === undefined ? [] : advertised.split(" ");
            if (object === undefined || name === undefined) {
                throw ForgeError.of(
                    "ADVERTISEMENT_UNEXPECTED",
                    `unexpected git remote advertisement: ${line}`,
                );
            }

            // read the default branch from HEAD's symref capability
            const symref = capabilities
                ?.split(" ")
                .find((capability) => capability.startsWith(HEAD_CAPABILITY));
            head = symref === undefined ? head : symref.slice(HEAD_CAPABILITY.length);

            // take the commit an annotated tag peels to
            if (name.endsWith(PEELED_SUFFIX)) {
                peeled.set(name.slice(0, -PEELED_SUFFIX.length), object);
            }
            // take branches and tags
            else if (name.startsWith("refs/heads/") || name.startsWith("refs/tags/")) {
                objects.set(name, object);
            }
        }

        // resolve each reference to its peeled commit, else to the object it points at
        const references: GitReference[] = [...objects].map(([name, object]) => ({
            name,
            object,
            commit: peeled.get(name) ?? object,
        }));

        return {
            defaultReference: head !== null && objects.has(head) ? head : null,
            references,
        };
    },

    /** Split a body into its pkt-lines' text, dropping flush packets. */
    packetLines(body: Uint8Array): string[] {
        // walk the packets from the start
        const decoder = new TextDecoder();
        const lines: string[] = [];
        let offset = 0;
        while (offset < body.length) {
            // read the length from its four hexadecimal digits, zero for a flush packet
            const prefix = decoder.decode(body.subarray(offset, offset + LENGTH_WIDTH));
            const length = Number.parseInt(prefix, 16);
            if (!/^[0-9a-f]{4}$/u.test(prefix) || (length !== 0 && length < LENGTH_WIDTH)) {
                throw ForgeError.of(
                    "PKT_LINE_UNEXPECTED",
                    `unexpected pkt-line from git remote: ${prefix}`,
                );
            }

            // take the payload of a data packet
            const end = length === 0 ? offset + LENGTH_WIDTH : offset + length;
            if (length !== 0) {
                lines.push(
                    decoder.decode(body.subarray(offset + LENGTH_WIDTH, end)).replace(/\n$/u, ""),
                );
            }
            offset = end;
        }

        return lines;
    },
};

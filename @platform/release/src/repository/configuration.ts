import { readFile } from "node:fs/promises";
import { ReleaseChannel } from "@destack/daemon/process";
import { Metadata, MetadataKind, type Root } from "@tufjs/models";
import { TrustedRoot } from "@destack/update/publish";
import { CHANNELS, type Channel } from "@destack/update/release";
import { parseDocument } from "./document.ts";

/** The host names of loopback rehearsal servers, which may serve plain HTTP. */
const LOOPBACK_HOSTS: ReadonlySet<string> = new Set(["localhost", "127.0.0.1", "[::1]"]);

/** A selected publication destination and its embedded initial trust. */
export class RepositoryConfiguration {
    /** The channel whose repository this configuration selects. */
    readonly channel: Channel;
    /** Public repository URL, including its feed directory. */
    readonly url: URL;
    /** Cloudflare account containing the release bucket. */
    readonly account: string;
    /** Bucket reserved for public releases. */
    readonly bucket: string;
    /** S3 endpoint used for conditional uploads and authoritative reads. */
    readonly endpoint: URL;

    /** Select a channel's repository, by default the environment's channel. */
    constructor(name = process.env["DESTACK_RELEASE_CHANNEL"]) {
        // require a channel for every signing and publication operation
        const channel = CHANNELS.find((candidate) => candidate === name);
        if (channel === undefined) {
            throw new Error("set DESTACK_RELEASE_CHANNEL to stable or nightly");
        }
        this.channel = channel;
        const origin = process.env["DESTACK_RELEASE_ORIGIN"];
        this.url =
            origin === undefined ? ReleaseChannel.of(channel).feed : new URL(`${channel}/`, origin);
        this.account = process.env["CLOUDFLARE_ACCOUNT_ID"] ?? "27c0d00fb3a27a4ccbf46a3cceab9301";
        this.bucket = process.env["DESTACK_RELEASE_BUCKET"] ?? `destack-releases-${channel}`;
        this.endpoint = new URL(
            process.env["DESTACK_RELEASE_S3_URL"] ??
                `https://${this.account}.r2.cloudflarestorage.com`,
        );

        // require encrypted remote storage and an unambiguous repository directory
        if (!isClean(this.endpoint) || this.endpoint.pathname !== "/") {
            throw new Error("release storage requires a clean HTTPS origin");
        }
        if (!isClean(this.url) || !this.url.pathname.endsWith("/")) {
            throw new Error("release repository requires a clean HTTPS directory URL");
        }
    }

    /** Read the selected public trust root, or an explicit rehearsal root. */
    async root(): Promise<Metadata<Root>> {
        // verify the complete initial quorum before using any online key
        const filename = this.channel === "stable" ? "root.json" : "nightly.json";
        const path =
            process.env["DESTACK_RELEASE_ROOT"] ??
            new URL(`../../../../@destack/cli/src/update/${filename}`, import.meta.url);
        const root = Metadata.fromJSON(
            MetadataKind.Root,
            parseDocument(await readFile(path, "utf8")),
        );
        TrustedRoot.authenticate(root);

        return root;
    }
}

/** Report whether a URL uses HTTPS, or HTTP on loopback, without credentials, query or fragment. */
function isClean(url: URL): boolean {
    const isLoopback = LOOPBACK_HOSTS.has(url.hostname);
    const isSecure = url.protocol === "https:" || (isLoopback && url.protocol === "http:");

    return (
        isSecure &&
        url.username === "" &&
        url.password === "" &&
        url.search === "" &&
        url.hash === ""
    );
}

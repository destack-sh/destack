import { readFile } from "node:fs/promises";
import { Metadata, MetadataKind, type Root } from "@tufjs/models";
import { authenticateRoot } from "../key/root.ts";

/** A selected publication destination and its embedded initial trust. */
export class RepositoryConfiguration {
    /** Stable or nightly release feed. */
    readonly channel: "stable" | "nightly";
    /** Public repository URL, including its feed directory. */
    readonly url: URL;
    /** Cloudflare account containing the release bucket. */
    readonly account: string;
    /** Bucket reserved for public releases. */
    readonly bucket: string;
    /** S3 endpoint used for conditional uploads and authoritative reads. */
    readonly endpoint: URL;

    /** Read the explicit feed and optional test publication destination. */
    constructor() {
        // require a feed for every signing and publication operation
        const channel = process.env.DESTACK_RELEASE_CHANNEL;
        if (channel !== "stable" && channel !== "nightly") {
            throw new Error("set DESTACK_RELEASE_CHANNEL to stable or nightly");
        }
        this.channel = channel;
        this.url = new URL(
            process.env.DESTACK_RELEASE_URL ?? `https://download.destack.sh/${channel}/`,
        );
        this.account = process.env.CLOUDFLARE_ACCOUNT_ID ?? "27c0d00fb3a27a4ccbf46a3cceab9301";
        this.bucket = process.env.DESTACK_RELEASE_BUCKET ?? `destack-releases-${channel}`;
        this.endpoint = new URL(
            process.env.DESTACK_RELEASE_S3_URL ??
                `https://${this.account}.r2.cloudflarestorage.com`,
        );

        // allow loopback rehearsal servers while requiring encrypted remote storage
        const isLocalStorage = ["localhost", "127.0.0.1", "[::1]"].includes(this.endpoint.hostname);
        if (
            (this.endpoint.protocol !== "https:" &&
                !(isLocalStorage && this.endpoint.protocol === "http:")) ||
            this.endpoint.username ||
            this.endpoint.password ||
            this.endpoint.search ||
            this.endpoint.hash ||
            this.endpoint.pathname !== "/"
        ) {
            throw new Error("release storage requires a clean HTTPS origin");
        }

        // reject ambiguous URL concatenation and insecure remote repositories
        const isLocal = ["localhost", "127.0.0.1", "[::1]"].includes(this.url.hostname);
        if (
            (this.url.protocol !== "https:" && !(isLocal && this.url.protocol === "http:")) ||
            !this.url.pathname.endsWith("/") ||
            this.url.search ||
            this.url.hash ||
            this.url.username ||
            this.url.password
        ) {
            throw new Error("release repository requires a clean HTTPS directory URL");
        }
    }

    /** Read the selected public trust root, or an explicit rehearsal root. */
    async root(): Promise<Metadata<Root>> {
        // verify the complete initial quorum before using any online key
        const filename = this.channel === "stable" ? "root.json" : "nightly.json";
        const path =
            process.env.DESTACK_RELEASE_ROOT ??
            new URL(`../../../../@destack/cli/src/update/${filename}`, import.meta.url);
        const root = Metadata.fromJSON(MetadataKind.Root, JSON.parse(await readFile(path, "utf8")));
        authenticateRoot(root);

        return root;
    }
}

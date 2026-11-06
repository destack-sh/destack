import type { Environment } from "@destack/environment";
import { Stage } from "@destack/update/release";
import { Metadata, MetadataKind, type Root } from "@tufjs/models";
import { TrustedRoot } from "@destack/update/publish";
import { RELEASE_ROOT } from "@destack/update/release";

/** The host names of loopback rehearsal servers, which may serve plain HTTP. */
const LOOPBACK_HOSTS: ReadonlySet<string> = new Set(["localhost", "127.0.0.1", "[::1]"]);

/** The root a rehearsal trusts in place of the feed's own. */
export interface RepositoryConfigurationOptions {
    /** The root a rehearsal or test trusts, absent for a release. */
    readonly root?: Metadata<Root>;
}

/** The feed's publication destination and its embedded initial trust. */
export class RepositoryConfiguration {
    /** Public repository URL, including its feed directory. */
    readonly url: URL;
    /** The root a rehearsal trusts, absent for a release. */
    readonly #root: Metadata<Root> | undefined;

    /** Select the feed, below a rehearsal's origin when the environment sets one. */
    constructor(environment: Environment<"process">, options: RepositoryConfigurationOptions = {}) {
        // address the feed, the same path below a rehearsal's origin
        const feed = Stage.of("production").feed;
        const origin = environment.process.variables["DESTACK_RELEASE_ORIGIN"];
        this.url = origin === undefined ? feed : new URL(feed.pathname.slice(1), origin);

        // require an unambiguous repository directory
        if (!RepositoryConfiguration.isClean(this.url) || !this.url.pathname.endsWith("/")) {
            throw new Error("release repository requires a clean HTTPS directory URL");
        }
        this.#root = options.root;
    }

    /** Read the root every release bundles, or a rehearsal's, with its whole quorum. */
    root(): Metadata<Root> {
        // read the rehearsal's root, else the feed's
        const root = this.#root ?? Metadata.fromJSON(MetadataKind.Root, RELEASE_ROOT);

        // verify the complete initial quorum before using any online key
        TrustedRoot.authenticate(root);

        return root;
    }

    /** Report whether a URL is HTTPS, or loopback HTTP, without credentials, query or fragment. */
    static isClean(url: URL): boolean {
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
}

import { DownloadHTTPError } from "tuf-js/dist/error";
import { Digest, Version } from "@destack/schema";
import { UpdateError } from "../error/error.ts";
import { join } from "node:path";
import { type TargetFile, Updater, type UpdaterOptions } from "tuf-js";
import { Release, type Target, TargetCustom } from "../release/release.ts";
import { type Environment, FileSystemError, type FileSystem } from "@destack/environment";
import { type DownloadOptions, Fetcher } from "./download.ts";

/** An authenticated release and its downloaded archive. */
export interface Download {
    /** Release described by signed target metadata. */
    release: Release;
    /** Verified local archive. */
    archive: string;
    /** Digest used to distinguish immutable distributions. */
    sha256: string;
}

/** A TUF repository with persistent trust and rollback protection. */
export class UpdateRepository {
    /** Metadata selected by this session, indexed by immutable release identity. */
    private readonly selected = new Map<string, TargetFile>();
    /** Metadata and download locations for target-specific fetchers. */
    private readonly locations: Pick<
        UpdaterOptions,
        "metadataDir" | "targetDir" | "metadataBaseUrl" | "targetBaseUrl"
    >;

    /** The clock, network and files metadata requests and archive downloads run on. */
    private readonly environment: Environment<"files">;

    /** Configure a repository after initializing its trusted root. */
    private constructor(directory: string, url: URL, environment: Environment<"files">) {
        this.environment = environment;
        this.locations = {
            metadataDir: join(directory, "metadata"),
            targetDir: join(directory, "downloads"),
            metadataBaseUrl: new URL("metadata/", url).href,
            targetBaseUrl: new URL("targets/", url).href,
        };
    }

    /** Initialize trust from the root bundled with the installed executable. */
    static async open(
        directory: string,
        url: URL,
        root: string,
        environment: Environment<"files">,
    ): Promise<UpdateRepository> {
        // permit plaintext only for local release verification
        const isLoopback = ["127.0.0.1", "[::1]", "localhost"].includes(url.hostname);
        if (url.protocol !== "https:" && !(url.protocol === "http:" && isLoopback)) {
            throw UpdateError.of("REPOSITORY_INVALID", "update repositories require HTTPS");
        }
        if (url.username || url.password || url.search || url.hash || !url.pathname.endsWith("/")) {
            throw UpdateError.of("REPOSITORY_INVALID", "invalid update repository URL");
        }

        // keep verified metadata across processes, safe to check then write under the installation lock
        const { files } = environment;
        await files.mkdir(join(directory, "metadata"), { recursive: true, mode: 0o700 });
        await files.mkdir(join(directory, "downloads"), { recursive: true, mode: 0o700 });
        await writeNew(files, join(directory, "metadata/root.json"), root);

        // prevent one cache from accepting metadata from multiple repositories
        const source = join(directory, "repository.txt");
        const isClaimed = !(await writeNew(files, source, url.href));
        if (isClaimed && (await files.readTextFile(source)) !== url.href) {
            throw UpdateError.of(
                "CACHE_MISMATCH",
                "update cache belongs to a different repository",
            );
        }

        return new UpdateRepository(directory, url, environment);
    }

    /** Refuse metadata the repository fails to serve or verify, keeping the update's refusals. */
    static #refused(error: unknown): Error {
        // keep a refusal of the update itself
        if (UpdateError.is(error)) {
            return error;
        }

        // refuse a failed download, else metadata that fails to verify, with TUF's reason
        const message = error instanceof Error ? error.message : String(error);
        const reason =
            error instanceof DownloadHTTPError ? "DOWNLOAD_FAILED" : "VERIFICATION_FAILED";

        return UpdateError.of(reason, `metadata refresh failed: ${message}`, { cause: error });
    }

    /** Refresh trusted metadata and describe the current release for a target. */
    async latest(target: Target): Promise<Release> {
        // refresh from the persisted trusted metadata
        const updater = new Updater({
            ...this.locations,
            fetcher: new Fetcher({ kind: "metadata" }, this.environment),
        });
        await updater.refresh().catch((error: unknown) => {
            throw UpdateRepository.#refused(error);
        });
        const artifact = await updater.getTargetInfo(`${target}.tar.gz`);
        if (!artifact) {
            throw UpdateError.of("RELEASE_NOT_FOUND", `no published release for ${target}`);
        }

        // read the signed release identity
        const custom = TargetCustom.safeParse(artifact.custom);
        if (!custom.success) {
            throw UpdateError.of("RELEASE_INVALID", "invalid release metadata");
        }

        // refuse a dev build, which no feed publishes
        const release = new Release(custom.data.version, target);
        if (Version.isPrerelease(release.version)) {
            throw UpdateError.of(
                "RELEASE_INVALID",
                `the feed publishes dev build ${release.version}`,
            );
        }

        // retain the exact signed target even if the repository publishes another release
        const previous = this.selected.get(release.directory);
        if (previous && !previous.equals(artifact)) {
            throw UpdateError.of("RELEASE_CHANGED", "published release changed");
        }
        this.selected.set(release.directory, artifact);

        return release;
    }

    /** Read the signed archive digest retained by the update check. */
    digest(release: Release): string {
        // read the digest selected by the update check
        const artifact = this.selected.get(release.directory);
        const digest = Digest.safeParse(artifact?.hashes["sha256"]);
        if (!digest.success) {
            throw UpdateError.of("RELEASE_INVALID", "invalid release archive digest");
        }

        return digest.data;
    }

    /** Download an archive and verify its signed size and digest before returning it. */
    async download(
        release: Release,
        candidate?: string,
        options: DownloadOptions = {},
    ): Promise<Download> {
        // require the selected release and its signed size and digest
        options.signal?.throwIfAborted();
        const artifact = this.selected.get(release.directory);
        if (!artifact || artifact.custom["version"] !== release.version) {
            throw UpdateError.of("RELEASE_CHANGED", "release changed after selection");
        }
        if (
            artifact.length > 2 * 1024 ** 3 ||
            artifact.hashes["sha256"] === undefined ||
            !Digest.safeParse(artifact.hashes["sha256"]).success
        ) {
            throw UpdateError.of("RELEASE_INVALID", "invalid release archive metadata");
        }

        // authenticate an installer archive by its signed length and hashes
        const { clock } = this.environment;
        const downloader = new Updater({
            ...this.locations,
            fetcher: new Fetcher(
                { kind: "archive", total: artifact.length, options },
                this.environment,
            ),
        });
        const cached = await clock.trackHost(downloader.findCachedTarget(artifact, candidate));
        if (candidate !== undefined && candidate !== "" && cached === undefined) {
            throw UpdateError.of("VERIFICATION_FAILED", "installer archive failed verification");
        }
        const archive = cached ?? (await downloader.downloadTarget(artifact));
        options.signal?.throwIfAborted();
        if (cached !== undefined) {
            options.onProgress?.({ received: artifact.length, total: artifact.length });
        }

        return { release, archive, sha256: artifact.hashes["sha256"] };
    }
}

/** Write a private file that does not exist yet, reporting whether it wrote one. */
async function writeNew(files: FileSystem, path: string, text: string): Promise<boolean> {
    try {
        await files.writeTextFile(path, text, { createNew: true, mode: 0o600 });

        return true;
    } catch (error) {
        if (!FileSystemError.is(error, "ALREADY_EXISTS")) {
            throw error;
        }

        return false;
    }
}

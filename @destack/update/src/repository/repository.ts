import { DownloadHTTPError } from "tuf-js/dist/error";
import { Digest, Version } from "@destack/schema";
import { CALL_ERROR_STATUSES } from "@destack/error";
import { UpdateError } from "../error/error.ts";
import { join } from "node:path";
import { type TargetFile, Updater, type UpdaterOptions } from "tuf-js";
import { Release, type Target, TargetCustom } from "../release/release.ts";
import { type Environment, FileSystemError, type FileSystem } from "@destack/environment";
import { type DownloadOptions, Fetcher } from "./download.ts";
import { Manifest } from "../manifest/manifest.ts";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";

/** The largest manifest the feed signs, in bytes: 16 MiB, far above a release's 2,000 files. */
const MANIFEST_MAX_BYTES = 16 * 1024 ** 2;

/** An authenticated release, its manifest and its files checked out in the download cache. */
export interface Download {
    /** Release described by signed target metadata. */
    release: Release;
    /** The release's signed manifest. */
    manifest: Manifest;
    /** The feed's blobs the manifest's files are fetched from. */
    blobs: URL;
    /** The directory with the release's files, verified against the manifest. */
    directory: string;
    /** The digest of the signed manifest, which identifies the release's files. */
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

    /** The clock, network and files metadata requests and downloads run on. */
    private readonly environment: Environment<"files">;
    /** The feed's blobs, each file's gzip bytes at its digest. */
    readonly blobs: URL;
    /** The directory with the downloaded manifests and each release's checked-out files. */
    private readonly downloads: string;
    /** The update's spans and durations. */
    readonly #scope: TelemetryScope;

    /** Configure a repository after initializing its trusted root. */
    private constructor(directory: string, url: URL, environment: Environment<"files">) {
        // locate the feed's blobs and metadata, and this cache's downloads
        this.environment = environment;
        this.#scope = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
        this.blobs = new URL("targets/blobs/", url);
        this.downloads = join(directory, "downloads");
        this.locations = {
            metadataDir: join(directory, "metadata"),
            targetDir: this.downloads,
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
            throw UpdateError.of(
                "REPOSITORY_INVALID",
                "update repository is invalid: it must use HTTPS",
            );
        }
        if (url.username || url.password || url.search || url.hash || !url.pathname.endsWith("/")) {
            throw UpdateError.of(
                "REPOSITORY_INVALID",
                "update repository URL is invalid: it must parse as a URL",
            );
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
                "CACHE_BLOCKED",
                "cannot use the update cache: it belongs to a different repository",
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

        // refuse missing metadata
        const message = error instanceof Error ? error.message : String(error);
        if (
            error instanceof DownloadHTTPError &&
            error.statusCode === CALL_ERROR_STATUSES.NOT_FOUND
        ) {
            return UpdateError.of("METADATA_NOT_FOUND", `metadata not found: ${message}`, {
                cause: error,
            });
        }
        // refuse a download the repository failed to answer
        else if (error instanceof DownloadHTTPError) {
            return UpdateError.of(
                "DOWNLOAD_UNREACHABLE",
                `could not download metadata: ${message}`,
                {
                    cause: error,
                },
            );
        }
        // refuse metadata that fails to verify, with TUF's reason
        else {
            return UpdateError.of("VERIFICATION_BROKEN", `could not verify metadata: ${message}`, {
                cause: error,
            });
        }
    }

    /** Open a TUF client fetching through a fetcher, its update fixed at the environment's now. */
    #updater(fetcher: Fetcher): Updater {
        // fix the update's start time (TUF client workflow 5.1) on the environment's clock
        const updater = new Updater({ ...this.locations, fetcher });
        const trusted: unknown = Reflect.get(updater, "trustedSet");
        if (typeof trusted !== "object" || trusted === null || !("referenceTime" in trusted)) {
            throw UpdateError.of("CLIENT_BROKEN", "tuf-js keeps no update start time");
        }
        Reflect.set(trusted, "referenceTime", new Date(this.environment.clock.now()));

        return updater;
    }

    /** Refresh trusted metadata and describe the current release for a target. */
    async latest(target: Target): Promise<Release> {
        // refresh from the persisted trusted metadata
        const updater = this.#updater(new Fetcher({ kind: "metadata" }, this.environment));
        await updater.refresh().catch((error: unknown) => {
            throw UpdateRepository.#refused(error);
        });
        const artifact = await updater.getTargetInfo(`${target}.json`);
        if (!artifact) {
            throw UpdateError.of(
                "RELEASE_NOT_FOUND",
                `published release for '${target}' not found`,
            );
        }

        // read the signed release identity
        const custom = TargetCustom.safeParse(artifact.custom);
        if (!custom.success) {
            throw UpdateError.of("RELEASE_BROKEN", "could not read the release metadata");
        }

        // refuse a dev build, which no feed publishes
        const release = new Release(custom.data.version, target);
        if (Version.isPrerelease(release.version)) {
            throw UpdateError.of(
                "RELEASE_BROKEN",
                `could not use dev build '${release.version}' from the feed`,
            );
        }

        // retain the exact signed target even if the repository publishes another release
        const previous = this.selected.get(release.directory);
        if (previous && !previous.equals(artifact)) {
            throw UpdateError.of(
                "RELEASE_CONFLICT",
                "cannot update: the published release changed",
            );
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
            throw UpdateError.of("RELEASE_BROKEN", "could not read the release manifest digest");
        }

        return digest.data;
    }

    /** Download a release's signed manifest, then check its files out into the download cache. */
    download(
        release: Release,
        sources: readonly string[],
        options: DownloadOptions = {},
    ): Promise<Download> {
        const attributes = { "destack.update.version": release.version };

        return this.#scope.span("destack.update.download", attributes, () =>
            this.#download(release, sources, options),
        );
    }

    /** Check a release's files out from its verified manifest, copying what the sources have. */
    async #download(
        release: Release,
        sources: readonly string[],
        options: DownloadOptions,
    ): Promise<Download> {
        // require the selected release and its signed size and digest
        options.signal?.throwIfAborted();
        const artifact = this.selected.get(release.directory);
        if (!artifact || artifact.custom["version"] !== release.version) {
            throw UpdateError.of(
                "RELEASE_CONFLICT",
                "cannot update: the release changed after selection",
            );
        }
        const sha256 = Digest.safeParse(artifact.hashes["sha256"]);
        if (artifact.length > MANIFEST_MAX_BYTES || !sha256.success) {
            throw UpdateError.of("RELEASE_BROKEN", "could not read the release manifest metadata");
        }

        // authenticate the manifest by its signed length and hashes, reading a cached copy first
        const { scheduler, files } = this.environment;
        const downloader = this.#updater(new Fetcher({ kind: "target" }, this.environment));
        const cached = await scheduler.run(() => downloader.findCachedTarget(artifact));
        const path = cached ?? (await downloader.downloadTarget(artifact));
        const manifest = Manifest.parse(await files.readTextFile(path));

        // check the files out beside the cache once, from the sources and the feed's blobs
        const directory = join(this.downloads, sha256.data);
        if (!(await files.exists(directory))) {
            const staging = await files.makeTemporaryDirectory(".download-", {
                directory: this.downloads,
            });
            try {
                await manifest.checkout(
                    staging,
                    { blobs: this.blobs, sources, ...options },
                    this.environment,
                );
                await files.rename(staging, directory);
            } finally {
                await files.remove(staging, { recursive: true, force: true });
            }
        }

        return { release, manifest, blobs: this.blobs, directory, sha256: sha256.data };
    }

    /** Remove a download's checked-out files once a staged release has them. */
    async discard(download: Download): Promise<void> {
        await this.environment.files.remove(download.directory, { recursive: true, force: true });
    }
}

/** Write a private file that does not exist yet, reporting whether it wrote one. */
async function writeNew(files: FileSystem, path: string, text: string): Promise<boolean> {
    try {
        await files.writeTextFile(path, text, { createNew: true, mode: 0o600 });

        return true;
    } catch (error) {
        if (!FileSystemError.is(error, "FILE_EXISTS")) {
            throw error;
        }

        return false;
    }
}

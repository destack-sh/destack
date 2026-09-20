import { UpdateError } from "../error/error.ts";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { type TargetFile, Updater } from "tuf-js";
import { Release, type Target } from "../release/release.ts";
import { DownloadFetcher, type DownloadOptions } from "./download.ts";

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
    private readonly locations: {
        metadataDir: string;
        targetDir: string;
        metadataBaseUrl: string;
        targetBaseUrl: string;
    };

    /** Configure a repository after initializing its trusted root. */
    private constructor(directory: string, url: URL) {
        this.locations = {
            metadataDir: join(directory, "metadata"),
            targetDir: join(directory, "downloads"),
            metadataBaseUrl: new URL("metadata/", url).href,
            targetBaseUrl: new URL("targets/", url).href,
        };
    }

    /** Initialize trust from the root bundled with the installed executable. */
    static async open(directory: string, url: URL, root: string): Promise<UpdateRepository> {
        // permit plaintext only for local release verification
        const isLoopback = ["127.0.0.1", "[::1]", "localhost"].includes(url.hostname);
        if (url.protocol !== "https:" && !(url.protocol === "http:" && isLoopback)) {
            throw new UpdateError("REPOSITORY", "Update repositories require HTTPS.");
        }
        if (url.username || url.password || url.search || url.hash || !url.pathname.endsWith("/")) {
            throw new UpdateError("REPOSITORY", "Invalid update repository URL.");
        }

        // retain previously verified metadata across process and release changes
        await mkdir(join(directory, "metadata"), { recursive: true, mode: 0o700 });
        await mkdir(join(directory, "downloads"), { recursive: true, mode: 0o700 });
        try {
            await writeFile(join(directory, "metadata/root.json"), root, {
                flag: "wx",
                mode: 0o600,
            });
        } catch (error) {
            if ((error as NodeJS.ErrnoException).code !== "EEXIST") throw error;
        }

        // prevent one cache from accepting metadata from multiple repositories
        const source = join(directory, "repository.txt");
        try {
            await writeFile(source, url.href, { flag: "wx", mode: 0o600 });
        } catch (error) {
            if ((error as NodeJS.ErrnoException).code !== "EEXIST") throw error;
            if (await readFile(source, "utf8") !== url.href) {
                throw new UpdateError(
                    "REPOSITORY",
                    "Update cache belongs to a different repository.",
                );
            }
        }

        return new UpdateRepository(directory, url);
    }

    /** Refresh trusted metadata and describe the current release for a target. */
    async latest(target: Target): Promise<Release> {
        // each TUF refresh starts from persisted trusted metadata
        const updater = new Updater({
            ...this.locations,
            config: { fetchTimeout: 30_000, fetchRetries: 2 },
        });
        await updater.refresh();
        const artifact = await updater.getTargetInfo(`${target}.tar.gz`);
        if (!artifact) throw new UpdateError("REPOSITORY", `No published release for ${target}.`);

        // retain the exact signed target even if the repository publishes another release
        const release = new Release(artifact.custom.version, target);
        const previous = this.selected.get(release.directory);
        if (previous && !previous.equals(artifact)) {
            throw new UpdateError("REPOSITORY", "Published release changed.");
        }
        this.selected.set(release.directory, artifact);

        return release;
    }

    /** Read the signed archive digest retained by the update check. */
    digest(release: Release): string {
        const artifact = this.selected.get(release.directory);
        const digest = artifact?.hashes.sha256;
        if (typeof digest !== "string" || !/^[0-9a-f]{64}$/.test(digest)) {
            throw new UpdateError("REPOSITORY", "Invalid release archive digest.");
        }

        return digest;
    }

    /** Download an archive and verify its signed size and digest before returning it. */
    async download(
        release: Release,
        candidate?: string,
        options: DownloadOptions = {},
    ): Promise<Download> {
        options.signal?.throwIfAborted();
        const artifact = this.selected.get(release.directory);
        if (!artifact || artifact.custom.version !== release.version) {
            throw new UpdateError("REPOSITORY", "Release changed after selection.");
        }
        if (
            artifact.length > 2 * 1024 ** 3 || !/^[0-9a-f]{64}$/.test(artifact.hashes.sha256 ?? "")
        ) {
            throw new UpdateError("REPOSITORY", "Invalid release archive metadata.");
        }
        // authenticate an installer archive using the same signed length and hashes
        const downloader = new Updater({
            ...this.locations,
            fetcher: new DownloadFetcher(artifact.length, options),
        });
        const cached = await downloader.findCachedTarget(artifact, candidate);
        if (candidate && !cached) {
            throw new UpdateError("REPOSITORY", "Installer archive failed verification.");
        }
        const archive = cached ?? await downloader.downloadTarget(artifact);
        options.signal?.throwIfAborted();
        if (cached) options.onProgress?.({ received: artifact.length, total: artifact.length });

        return { release, archive, sha256: artifact.hashes.sha256 };
    }
}

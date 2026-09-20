import type { Release } from "../release/release.ts";
import type { Download, UpdateRepository } from "../repository/repository.ts";
import type { DownloadOptions } from "../repository/download.ts";

/** An authenticated release selected during an update check. */
export class Update {
    /** Release selected by the check. */
    readonly release: Release;
    /** Repository containing the selected archive. */
    private readonly repository: UpdateRepository;
    /** Acquire exclusive access to the session's cache. */
    private readonly begin: () => Disposable;

    /** Retain the selected release and its locked repository. */
    constructor(release: Release, repository: UpdateRepository, begin: () => Disposable) {
        this.release = release;
        this.repository = repository;
        this.begin = begin;
    }

    /** Download and authenticate the release, optionally using a bundled installer archive. */
    async download(options: DownloadOptions = {}): Promise<Download> {
        using operation = this.begin();

        return await this.repository.download(this.release, options.archive, options);
    }
}

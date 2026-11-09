import type { Release } from "../release/release.ts";
import type { Download, UpdateRepository } from "../repository/repository.ts";
import type { DownloadOptions } from "../repository/download.ts";

/** An authenticated release selected during an update check. */
export class Update {
    /** Release selected by the check. */
    readonly release: Release;
    /** Repository containing the selected release. */
    private readonly repository: UpdateRepository;
    /** The installed distribution's directory, whose files the download copies first. */
    private readonly installed: string | undefined;
    /** Acquire exclusive access to the session's cache. */
    private readonly begin: () => Disposable;

    /** Retain the selected release, its locked repository and the installed distribution. */
    constructor(
        release: Release,
        repository: UpdateRepository,
        installed: string | undefined,
        begin: () => Disposable,
    ) {
        // keep the release, the repository, the installed files and the session's lock
        this.release = release;
        this.repository = repository;
        this.installed = installed;
        this.begin = begin;
    }

    /** Download and authenticate the release, copying what the installed or a given source has. */
    async download(options: DownloadOptions = {}): Promise<Download> {
        // read the installed distribution's files and the given source's before the feed
        using operation = this.begin();
        const sources = [this.installed, options.source].filter((each) => each !== undefined);

        return await this.repository.download(this.release, sources, options);
    }
}

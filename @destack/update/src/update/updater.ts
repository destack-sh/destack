import { UpdateError } from "../error/error.ts";
import { isAbsolute, join } from "node:path";
import { type InstalledRelease, Installer, type StagedRelease } from "../install/installer.ts";
import { Release, type Target } from "../release/release.ts";
import { type Download, UpdateRepository } from "../repository/repository.ts";
import { Update } from "./update.ts";
import { FileLock } from "@destack/fs";
import { mkdir, stat } from "node:fs/promises";

/** Maximum time allowed for a staged executable to report its version, in milliseconds. */
const VERIFY_TIMEOUT_MS = 60_000;

/** A locked update session for one installed Destack distribution. */
export class Updater implements AsyncDisposable {
    /** Distribution files and activation records. */
    private readonly installer: Installer;
    /** Verified update repository. */
    private readonly repository: UpdateRepository;
    /** Exclusive metadata and installation lock. */
    readonly lock: FileLock;
    /** Platform and application selected by the caller. */
    private readonly options: UpdaterOptions;
    /** Whether this session has released its lock. */
    private isClosed = false;
    /** Whether an operation is accessing metadata or installation files. */
    private operation?: PromiseWithResolvers<void>;

    /** Construct an updater after acquiring its installation lock. */
    private constructor(options: UpdaterOptions, lock: FileLock, repository: UpdateRepository) {
        // retain the options, lock and repository
        this.options = { ...options };
        this.lock = lock;
        this.repository = repository;
        this.installer = new Installer(options.directory);
    }

    /** Lock the installation and open trusted metadata. */
    static async open(options: UpdaterOptions): Promise<Updater> {
        // reject inconsistent installation inputs before opening or recovering state
        if (!isAbsolute(options.directory)) {
            throw new UpdateError("INSTALL", "Installation directory must be absolute.");
        }
        const isMac = options.target.endsWith("apple-darwin");
        if (
            isMac !== (options.application !== undefined) ||
            (isMac && !options.applicationIdentifier) ||
            (options.application !== undefined && !isAbsolute(options.application))
        ) {
            throw new UpdateError(
                "INSTALL",
                "macOS releases require an absolute application destination and identifier",
            );
        }
        if (options.current && options.current.target !== options.target) {
            throw new UpdateError(
                "RELEASE",
                "Running release does not match the installation target.",
            );
        }

        // serialize trust updates and installation across CLI and desktop processes
        await mkdir(options.directory, { recursive: true, mode: 0o700 });
        const lock = await FileLock.tryAcquire(`${options.directory}.lock`);
        if (!lock) {
            throw new UpdateError("BUSY", "Another Destack update is running.");
        }
        try {
            const repository = await UpdateRepository.open(
                join(options.directory, "update"),
                options.repository,
                options.root,
            );
            const updater = new Updater(options, lock, repository);
            return updater;
        } catch (error) {
            await lock.close();
            throw error;
        }
    }

    /** Read the installed distribution. */
    async current(): Promise<InstalledRelease | undefined> {
        using operation = this.begin();

        return await this.installer.current();
    }

    /** Read a release prepared by an earlier update session. */
    async staged(): Promise<StagedRelease | undefined> {
        using operation = this.begin();

        return await this.installer.staged();
    }

    /** Check for a newer authenticated release, or a first installation. */
    async check(): Promise<Update | undefined> {
        // read the latest and installed releases on this target
        using operation = this.begin();
        const latest = await this.repository.latest(this.options.target);
        const installed = await this.installer.current();
        let current = installed?.release;
        if (current && current.target !== this.options.target) {
            throw new UpdateError("INSTALL", "Cannot change the installed platform.");
        }
        if (
            installed &&
            latest.compare(installed.release) === 0 &&
            this.repository.digest(latest) !== installed.sha256
        ) {
            throw new UpdateError("RELEASE", "Published release changed.");
        }

        // include a running distribution opened directly from a downloaded application
        if (this.options.current && (!current || this.options.current.compare(current) > 0)) {
            current = this.options.current;
        }
        if (current && latest.compare(current) < 0) {
            throw new UpdateError(
                "RELEASE",
                "Published release is older than the installed version.",
            );
        }

        return current && latest.compare(current) === 0
            ? undefined
            : new Update(latest, this.repository, () => this.begin());
    }

    /** Verify and stage a downloaded distribution while the current release remains active. */
    async stage(download: Download): Promise<StagedRelease> {
        // require the downloaded release for this target
        using operation = this.begin();
        if (download.release.target !== this.options.target) {
            throw new UpdateError(
                "RELEASE",
                "Downloaded release does not match the installation target.",
            );
        }
        if (this.options.current && download.release.compare(this.options.current) < 0) {
            throw new UpdateError(
                "RELEASE",
                "Refusing to stage a release older than the running version.",
            );
        }

        // authenticate the archive again before extracting or executing its contents
        const verified = await this.repository.download(download.release, download.archive);
        const previous = await this.installer.current();
        const installed = await this.installer.stage(verified, (directory) =>
            verifyRelease(directory, verified.release),
        );
        const staged = { ...installed, previous: previous?.sha256 };
        await this.installer.remember(staged);

        return staged;
    }

    /** Activate a staged distribution after callers have stopped affected processes. */
    async activate(staged: StagedRelease): Promise<InstalledRelease> {
        // refuse a release older than the running version
        using operation = this.begin();
        if (this.options.current && staged.release.compare(this.options.current) < 0) {
            throw new UpdateError(
                "RELEASE",
                "Refusing to activate a release older than the running version.",
            );
        }

        // refresh trust after staging may have waited outside an update session
        const latest = await this.repository.latest(this.options.target);
        if (
            latest.compare(staged.release) !== 0 ||
            latest.target !== staged.release.target ||
            this.repository.digest(latest) !== staged.sha256
        ) {
            throw new UpdateError("RELEASE", "Staged release is no longer the published release.");
        }
        const current = await this.installer.current();
        if (current?.sha256 !== staged.previous && current?.sha256 !== staged.sha256) {
            throw new UpdateError("INSTALL", "Installation changed after staging.");
        }

        // reuse the authenticated cache and derive paths from the installation directory
        const download = await this.repository.download(latest);
        const installed = await this.installer.stage(download, (directory) =>
            verifyRelease(directory, latest),
        );
        await this.installer.activate(
            installed,
            this.options.application,
            this.options.applicationIdentifier,
        );

        return installed;
    }

    /** Release the installation lock. */
    async [Symbol.asyncDispose](): Promise<void> {
        this.isClosed = true;
        await this.operation?.promise;
        await this.lock.close();
    }

    /** Serialize operations and retain the lock until in-progress work finishes. */
    private begin(): Disposable {
        if (this.isClosed) {
            throw new UpdateError("CLOSED", "Update session is closed.");
        }
        if (this.operation) {
            throw new UpdateError("BUSY", "An update operation is already running.");
        }
        const operation = Promise.withResolvers<void>();
        this.operation = operation;

        return {
            [Symbol.dispose]: () => {
                this.operation = undefined;
                operation.resolve();
            },
        };
    }
}

/** Inputs for one distribution's update session. */
export interface UpdaterOptions {
    /** Absolute directory containing versioned distributions and persisted trust. */
    directory: string;
    /** TUF repository containing metadata and targets. */
    repository: URL;
    /** Initial public root metadata bundled with the executable. */
    root: string;
    /** Platform of the distribution to install. */
    target: Target;
    /** Running release, including applications launched outside a managed installation. */
    current?: Release;
    /** Absolute macOS application destination. */
    application?: string;
    /** Expected macOS bundle identifier, required when an application destination is supplied. */
    applicationIdentifier?: string;
}

/** Check staged executable identity and the presence of its desktop application. */
async function verifyRelease(directory: string, release: Release): Promise<void> {
    // execute only an archive previously authenticated by the update repository
    const name = release.target.includes("windows") ? "destack.exe" : "destack";
    const child = Bun.spawn([join(directory, "bin", name), "version", "--json"], {
        env: { ...process.env, DESTACK_UPDATE_CHECK: "1" },
        timeout: VERIFY_TIMEOUT_MS,
        stdout: "pipe",
        stderr: "pipe",
    });
    const [code, stdout, stderr] = await Promise.all([
        child.exited,
        new Response(child.stdout).text(),
        new Response(child.stderr).text(),
    ]);
    if (code !== 0) {
        const diagnostic = stderr.trim();
        throw new UpdateError(
            "RELEASE",
            `Downloaded CLI exited with ${child.signalCode ?? code}: ${diagnostic}`,
        );
    }

    // reject incomplete distributions before changing the active release
    if (JSON.parse(stdout).version !== release.version) {
        throw new UpdateError(
            "RELEASE",
            "Downloaded CLI version does not match the signed release.",
        );
    }
    const desktop = release.target.endsWith("apple-darwin") ? "Destack.app" : "Destack";
    if (!(await stat(join(directory, desktop))).isDirectory()) {
        throw new UpdateError("RELEASE", "Downloaded desktop is missing.");
    }
}

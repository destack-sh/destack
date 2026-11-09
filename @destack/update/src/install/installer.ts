import { type Environment, type FileLock, FileSystemError } from "@destack/environment";
import { Digest, schema, Uuid } from "@destack/schema";
import { UpdateError } from "../error/error.ts";
import { dirname, isAbsolute, join, resolve } from "node:path";
import type { Download } from "../repository/repository.ts";
import { Release } from "../release/release.ts";
import { installApplication } from "../apple/install.ts";

/** The mode of a device's private directories: the owner's alone. */
export const PRIVATE_DIRECTORY_MODE = 0o700;

/** The mode of a device's private files: the owner's alone. */
const PRIVATE_FILE_MODE = 0o600;

/** The persisted record of the current release. */
const CurrentRecord = schema.object({
    /** The calendar version. */
    version: schema.string(),
    /** The operating system and architecture. */
    target: schema.string(),
    /** The archive digest. */
    sha256: Digest,
});

/** The persisted record of a staged release. */
const StagedRecord = CurrentRecord.extend({
    /** The archive digest installed before staging, when present. */
    previous: Digest.exactOptional(),
});

/** The persisted intent to activate a staged release. */
const ActivationRecord = CurrentRecord.extend({
    /** The absolute native application destination, for macOS releases. */
    application: schema.string().exactOptional(),
    /** The native application bundle identifier, for macOS releases. */
    applicationIdentifier: schema.string().exactOptional(),
});

/** The receipt written into a verified distribution directory. */
const Receipt = schema.object({
    /** The archive digest the distribution was extracted from. */
    sha256: Digest,
});

/** A complete distribution selected by an atomic installation record. */
export interface InstalledRelease {
    /** Installed calendar release. */
    release: Release;
    /** Immutable archive digest. */
    sha256: string;
    /** Absolute distribution directory. */
    directory: string;
}

/** A verified distribution prepared for activation. */
export interface StagedRelease extends InstalledRelease {
    /** Installed release observed before staging, when present. */
    previous?: string;
}

/** Versioned application files kept separately from user spaces and databases. */
export class Installer {
    /** Directory containing release versions and the current installation record. */
    readonly directory: string;
    /** The files and randomness the installation is kept with. */
    readonly environment: Environment<"files" | "process">;

    /** Select the installation managed by the local host, kept through an environment's files. */
    constructor(directory: string, environment: Environment<"files" | "process">) {
        this.directory = resolve(directory);
        this.environment = environment;
    }

    /** Take the installation lock beside the directory, refusing a concurrent installation. */
    async lock(): Promise<FileLock> {
        // create the private parent directory and take the lock file beside the directory
        const { files } = this.environment;
        await files.mkdir(dirname(this.directory), {
            recursive: true,
            mode: PRIVATE_DIRECTORY_MODE,
        });
        const lock = await files.tryLock(`${this.directory}.lock`, "exclusive");
        if (!lock) {
            throw UpdateError.of(
                "UPDATE_RUNNING",
                "another Destack installation operation is running",
            );
        }

        return lock;
    }

    /** Retain a staged release across updater sessions. */
    async remember(staged: StagedRelease): Promise<void> {
        // write the staged record atomically
        const record = {
            version: staged.release.version,
            target: staged.release.target,
            sha256: staged.sha256,
            previous: staged.previous,
        };
        await this.#replace("staged.json", JSON.stringify(record));
    }

    /** Read the distribution waiting for an explicit restart. */
    async staged(): Promise<StagedRelease | undefined> {
        // read the staged release record, absent when none is staged
        const source = await this.#read("staged.json");
        if (source === undefined) {
            return undefined;
        }

        // validate persisted identifiers before constructing installation paths
        const record = StagedRecord.safeParse(JSON.parse(source));
        if (!record.success) {
            throw UpdateError.of("RECORD_INVALID", "invalid staged release record");
        }
        const { version, target, ...digests } = record.data;
        const release = new Release(version, target);

        return { release, ...digests, directory: this.path(release) };
    }

    /** Read the current release without modifying application files. */
    async current(): Promise<InstalledRelease | undefined> {
        // read the current release record, absent before installation
        const source = await this.#read("current.json");
        if (source === undefined) {
            return undefined;
        }

        // validate the persisted release before constructing its path
        const record = CurrentRecord.safeParse(JSON.parse(source));
        if (!record.success) {
            throw UpdateError.of("RECORD_INVALID", "invalid installed release record");
        }
        const release = new Release(record.data.version, record.data.target);

        return { release, sha256: record.data.sha256, directory: this.path(release) };
    }

    /** Check a downloaded distribution out and check its executables before activation. */
    async stage(
        download: Download,
        check: (directory: string) => Promise<void>,
    ): Promise<InstalledRelease> {
        // reject downgrades and reuse only an identical authenticated release
        const current = await this.current();
        if (current && current.release.target !== download.release.target) {
            throw UpdateError.of("PLATFORM_CHANGED", "cannot change the installed platform");
        }
        if (current && download.release.compare(current.release) < 0) {
            throw UpdateError.of("DOWNGRADE_REFUSED", "cannot install an older release");
        }
        if (current && download.release.compare(current.release) === 0) {
            if (current.sha256 !== download.sha256) {
                throw UpdateError.of("RELEASE_CHANGED", "published release changed");
            }
            await check(current.directory);
            return current;
        }

        // check out into a fresh sibling directory without touching the active distribution
        const { files, random } = this.environment;
        const versions = join(this.directory, "versions");
        await files.mkdir(versions, { recursive: true, mode: PRIVATE_DIRECTORY_MODE });
        const staging = join(versions, `.install-${Uuid.v4(random)}`);
        await files.mkdir(staging, { mode: PRIVATE_DIRECTORY_MODE });
        try {
            await download.manifest.checkout(
                staging,
                { blobs: download.blobs, sources: [download.directory] },
                this.environment,
            );
            await check(staging);

            // retain verified releases after interrupted activation without overwriting them
            const destination = this.path(download.release);
            const receipt = JSON.stringify({ sha256: download.sha256 });
            await files.writeTextFile(join(staging, "receipt.json"), receipt, {
                createNew: true,
                mode: PRIVATE_FILE_MODE,
            });
            const isPresent = await files.rename(staging, destination).then(
                () => false,
                (error: unknown) => {
                    if (!FileSystemError.is(error, "ALREADY_EXISTS")) {
                        throw error;
                    }

                    return true;
                },
            );
            if (isPresent) {
                if ((await files.readTextFile(join(destination, "receipt.json"))) !== receipt) {
                    throw UpdateError.of(
                        "VERIFICATION_FAILED",
                        "installed release directory contains different content",
                    );
                }
                await check(destination);
            }

            return { release: download.release, sha256: download.sha256, directory: destination };
        } finally {
            // remove the staging directory left behind by a failure or a release already present
            await files.remove(staging, { recursive: true, force: true });
        }
    }

    /** Record activation before replacing application files so interrupted updates can resume. */
    async activate(
        installed: InstalledRelease,
        application?: string,
        applicationIdentifier?: string,
    ): Promise<void> {
        // reject stale activation before recording its intent
        await this.checkActivation(installed.release, installed.sha256);

        // require a destination for native macOS bundles
        const isMac = installed.release.target.endsWith("apple-darwin");
        const isIdentified = applicationIdentifier !== undefined && applicationIdentifier !== "";
        if (isMac !== (application !== undefined) || (isMac && !isIdentified)) {
            throw UpdateError.of(
                "OPTIONS_INVALID",
                "macOS releases require an application destination and identifier",
            );
        }

        // persist the intended release before changing either active location
        const record = {
            version: installed.release.version,
            target: installed.release.target,
            sha256: installed.sha256,
            application,
            applicationIdentifier,
        };
        await this.#replace("activate.json", JSON.stringify(record));
        try {
            await this.recover();
        } catch (error) {
            throw UpdateError.of(
                "ACTIVATION_FAILED",
                "activation did not finish; retry `destack self activate` before restarting applications",
                { cause: error },
            );
        }
    }

    /** Complete a recorded activation after affected processes have stopped. */
    async recover(): Promise<void> {
        // read only activations that were committed to the installation directory
        const { files } = this.environment;
        const source = await this.#read("activate.json");
        if (source === undefined) {
            return;
        }

        // verify the staged receipt before replacing the native application
        const parsed = ActivationRecord.safeParse(JSON.parse(source));
        if (!parsed.success) {
            throw UpdateError.of("RECORD_INVALID", "invalid activation record");
        }
        const record = parsed.data;
        const release = new Release(record.version, record.target);
        const directory = this.path(release);
        const receipt = Receipt.safeParse(
            JSON.parse(await files.readTextFile(join(directory, "receipt.json"))),
        );
        if (!receipt.success || receipt.data.sha256 !== record.sha256) {
            throw UpdateError.of(
                "VERIFICATION_FAILED",
                "activation receipt does not match the staged release",
            );
        }
        await this.checkActivation(release, record.sha256);
        if (release.target.endsWith("apple-darwin")) {
            const { application, applicationIdentifier } = record;
            if (
                application === undefined ||
                !isAbsolute(application) ||
                applicationIdentifier === undefined ||
                applicationIdentifier === ""
            ) {
                throw UpdateError.of(
                    "RECORD_INVALID",
                    "missing absolute application destination or identifier",
                );
            }
            await installApplication(
                join(directory, "Destack.app"),
                application,
                applicationIdentifier,
                this.environment,
            );
        }

        // select the release only after platform installation succeeds
        const current =
            JSON.stringify({
                version: release.version,
                target: release.target,
                sha256: record.sha256,
            }) + "\n";
        await this.#replace("current.json", current);
        await files.remove(join(this.directory, "staged.json"), { force: true });
        await files.remove(join(this.directory, "activate.json"));
    }

    /** Read a record of the installation directory, absent when none was published. */
    async #read(name: string): Promise<string | undefined> {
        const { files } = this.environment;

        return await files.readTextFile(join(this.directory, name)).catch(FileSystemError.absent);
    }

    /** Replace a record of the installation directory whole, written aside and renamed in. */
    async #replace(name: string, text: string): Promise<void> {
        // write it under a fresh name, then rename it into place
        const { files, random } = this.environment;
        const pending = join(this.directory, `${name}.${Uuid.v4(random)}`);
        await files.writeTextFile(pending, text, { createNew: true, mode: PRIVATE_FILE_MODE });
        await files.rename(pending, join(this.directory, name));
    }

    /** Resolve a validated release beneath the installation directory. */
    private path(release: Release): string {
        return join(this.directory, "versions", release.directory);
    }

    /** Reject activation that would replace a newer or different installed release. */
    private async checkActivation(release: Release, sha256: string): Promise<void> {
        const current = await this.current();
        if (
            current &&
            (current.release.target !== release.target || current.release.compare(release) > 0)
        ) {
            throw UpdateError.of(
                "DOWNGRADE_REFUSED",
                "cannot activate an older or incompatible release",
            );
        }
        if (current && current.release.compare(release) === 0 && current.sha256 !== sha256) {
            throw UpdateError.of("RELEASE_CHANGED", "published release changed");
        }
    }
}

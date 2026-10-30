import {
    type Environment,
    type FileLock,
    type FileSystem,
    FileSystemError,
} from "@destack/environment";
import type { FamilyError } from "@destack/error";
import { Digest, present, schema, Uuid } from "@destack/schema";
import { UpdateError, type UpdateReason } from "../error/error.ts";
import { dirname, isAbsolute, join, relative, resolve, sep } from "node:path";
import { Parser, type ReadEntry } from "tar";
import type { Download } from "../repository/repository.ts";
import { Release } from "../release/release.ts";
import { installApplication } from "../apple/install.ts";

/** The mode of a device's private directories: the owner's alone. */
export const PRIVATE_DIRECTORY_MODE = 0o700;

/** The mode of a device's private files: the owner's alone. */
const PRIVATE_FILE_MODE = 0o600;

/** The permission bits of a mode, without its file type. */
const PERMISSION_BITS = 0o777;

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

    /** Extract a verified distribution and check its executables before activation. */
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

        // extract into a fresh sibling directory without touching the active distribution
        const { files, random } = this.environment;
        const versions = join(this.directory, "versions");
        await files.mkdir(versions, { recursive: true, mode: PRIVATE_DIRECTORY_MODE });
        const staging = join(versions, `.install-${Uuid.v4(random)}`);
        await files.mkdir(staging, { mode: PRIVATE_DIRECTORY_MODE });
        try {
            await extractArchive(this.environment, download.archive, staging);
            await verifyLinks(files, staging, staging);
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

/** Extract an archive's safe entries under a directory, refusing an unsafe one. */
async function extractArchive(
    environment: Environment<"files">,
    archive: string,
    root: string,
): Promise<void> {
    // write each safe entry in turn, draining every entry after the first refusal or failed write
    const { scheduler, files } = environment;
    let rejected: FamilyError<UpdateReason> | undefined;
    let written = Promise.resolve();
    const parser = new Parser({
        strict: true,
        onReadEntry: (entry) => {
            rejected ??= rejectEntry(entry.path, entry, root);
            const drain = (error: unknown): never => {
                entry.resume();
                throw error;
            };
            written = written.then(async () => {
                if (rejected === undefined) {
                    await writeEntry(files, root, entry).catch(drain);
                } else {
                    entry.resume();
                }
            }, drain);
        },
    });
    const parsed = new Promise<void>((finish, fail) => {
        parser.on("end", finish);
        parser.on("error", fail);
    });

    // stream the archive into the parser with backpressure, its drain waiting on the world's writes
    await using handle = await files.open(archive, { read: true });
    for await (const chunk of handle.readable()) {
        const bytes = Buffer.from(chunk.buffer, chunk.byteOffset, chunk.byteLength);
        if (!parser.write(bytes)) {
            await scheduler.track(
                new Promise<void>((drained) => {
                    parser.once("drain", drained);
                }),
                "peer",
            );
        }
    }
    parser.end();
    await scheduler.track(parsed, "peer");
    await written;

    // refuse the first unsafe entry once the archive drained
    if (rejected !== undefined) {
        throw rejected;
    }
}

/** Write one safe archive entry under a directory, streaming a file's bytes as they arrive. */
async function writeEntry(files: FileSystem, root: string, entry: ReadEntry): Promise<void> {
    // create the entry's parent, then the entry by its type, draining a link's or directory's empty body
    const path = join(root, entry.path);
    await files.mkdir(dirname(path), { recursive: true });
    if (entry.type === "SymbolicLink") {
        entry.resume();
        await files.symlink(present(entry.linkpath, `the link target of ${entry.path}`), path);
    } else if (entry.type === "Directory") {
        entry.resume();
        const mode = present(entry.mode, `the mode of ${entry.path}`) & PERMISSION_BITS;
        await files.mkdir(path, { recursive: true, mode });
    } else {
        const mode = present(entry.mode, `the mode of ${entry.path}`) & PERMISSION_BITS;
        await using handle = await files.open(path, { write: true, createNew: true, mode });
        const writer = handle.writable().getWriter();
        for await (const chunk of entry) {
            await writer.write(new Uint8Array(chunk));
        }
        await writer.close();
    }
}

/** Reject symlinks that escape the extracted distribution. */
async function verifyLinks(files: FileSystem, root: string, directory: string): Promise<void> {
    for (const { name } of await files.readDirectory(directory)) {
        const path = join(directory, name);
        const stat = await files.lstat(path);
        if (stat.kind === "symlink") {
            const link = await files.readLink(path);
            const destination = resolve(directory, link);
            const relation = relative(root, destination);
            if (isAbsolute(link) || relation === ".." || relation.startsWith(`..${sep}`)) {
                throw UpdateError.of(
                    "UNSAFE_ARCHIVE",
                    `archive link escapes the distribution: ${path}`,
                );
            }
        } else if (stat.kind === "directory") {
            await verifyLinks(files, root, path);
        }
    }
}

/** Find why an archive entry could write outside the staged distribution, if it could. */
function rejectEntry(
    path: string,
    entry: ReadEntry,
    root: string,
): FamilyError<UpdateReason> | undefined {
    // split the path on both separators
    const components = path.replaceAll("\\", "/").split("/");

    // require a relative path inside the distribution
    if (isAbsolute(path) || components.includes("..") || /^[A-Za-z]:/u.test(path)) {
        return UpdateError.of("UNSAFE_ARCHIVE", `unsafe archive path: ${path}`);
    }
    // require a mode, which the extracted files and directories keep
    else if (entry.mode === undefined && entry.type !== "SymbolicLink") {
        return UpdateError.of("UNSAFE_ARCHIVE", `archive entry has no mode: ${path}`);
    }
    // require a supported entry type
    else if (!["File", "Directory", "SymbolicLink"].includes(entry.type)) {
        return UpdateError.of(
            "UNSAFE_ARCHIVE",
            `unsupported archive entry: ${path} (${entry.type})`,
        );
    }
    // accept files and directories
    else if (entry.type !== "SymbolicLink") {
        return undefined;
    }

    // require a link target inside the distribution
    if (entry.linkpath === undefined || entry.linkpath === "") {
        return UpdateError.of("UNSAFE_ARCHIVE", `missing archive link target: ${path}`);
    }
    const target = resolve(root, path, "..", entry.linkpath);
    const relation = relative(root, target);
    if (isAbsolute(entry.linkpath) || relation === ".." || relation.startsWith(`..${sep}`)) {
        return UpdateError.of("UNSAFE_ARCHIVE", `archive link escapes the distribution: ${path}`);
    }

    return undefined;
}

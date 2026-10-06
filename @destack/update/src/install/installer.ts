import { readOptional } from "@destack/fs";
import { Digest, schema } from "@destack/schema";
import { UpdateError } from "../error/error.ts";
import {
    lstat,
    mkdir,
    mkdtemp,
    readdir,
    readFile,
    readlink,
    rename,
    rm,
    writeFile,
} from "node:fs/promises";
import { isAbsolute, join, relative, resolve, sep } from "node:path";
import { extract, type ReadEntry } from "tar";
import type { Stats } from "node:fs";
import type { Download } from "../repository/repository.ts";
import { Release } from "../release/release.ts";
import { installApplication } from "../apple/install.ts";

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

    /** Select the installation managed by the local host. */
    constructor(directory: string) {
        this.directory = resolve(directory);
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
        const temporary = join(this.directory, `staged.${crypto.randomUUID()}.json`);
        await writeFile(temporary, JSON.stringify(record), { flag: "wx", mode: 0o600 });
        await rename(temporary, join(this.directory, "staged.json"));
    }

    /** Read the distribution waiting for an explicit restart. */
    async staged(): Promise<StagedRelease | undefined> {
        // read the staged release record, absent when none is staged
        const source = await readOptional(join(this.directory, "staged.json"));
        if (source === undefined) {
            return undefined;
        }

        // validate persisted identifiers before constructing installation paths
        const record = StagedRecord.safeParse(JSON.parse(source));
        if (!record.success) {
            throw new UpdateError("INSTALL", "invalid staged release record");
        }
        const { version, target, ...digests } = record.data;
        const release = new Release(version, target);

        return { release, ...digests, directory: this.path(release) };
    }

    /** Read the current release without modifying application files. */
    async current(): Promise<InstalledRelease | undefined> {
        // read the current release record, absent before installation
        const source = await readOptional(join(this.directory, "current.json"));
        if (source === undefined) {
            return undefined;
        }

        // validate the persisted release before constructing its path
        const record = CurrentRecord.safeParse(JSON.parse(source));
        if (!record.success) {
            throw new UpdateError("INSTALL", "invalid installed release record");
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
            throw new UpdateError("INSTALL", "cannot change the installed platform");
        }
        if (current && download.release.compare(current.release) < 0) {
            throw new UpdateError("INSTALL", "refusing to install an older release");
        }
        if (current && download.release.compare(current.release) === 0) {
            if (current.sha256 !== download.sha256) {
                throw new UpdateError("INSTALL", "published release changed");
            }
            await check(current.directory);
            return current;
        }

        // extract into a fresh sibling directory without touching the active distribution
        const versions = join(this.directory, "versions");
        await mkdir(versions, { recursive: true, mode: 0o700 });
        const staging = await mkdtemp(join(versions, ".install-"));
        try {
            let rejected: UpdateError | undefined;
            await extract({
                file: download.archive,
                cwd: staging,
                strict: true,
                preservePaths: false,
                filter(path, entry) {
                    // keep the first rejection and fail after the parser drains
                    rejected ??= rejectEntry(path, entry, staging);
                    return rejected === undefined;
                },
            });
            if (rejected !== undefined) {
                throw rejected;
            }
            await verifyLinks(staging, staging);
            await check(staging);

            // retain verified releases after interrupted activation without overwriting them
            const destination = this.path(download.release);
            const receipt = JSON.stringify({ sha256: download.sha256 });
            await writeFile(join(staging, "receipt.json"), receipt, { flag: "wx", mode: 0o600 });
            const isPresent = await rename(staging, destination).then(
                () => false,
                (error: NodeJS.ErrnoException) => {
                    if (error.code !== "EEXIST" && error.code !== "ENOTEMPTY") {
                        throw error;
                    }
                    return true;
                },
            );
            if (isPresent) {
                if ((await readFile(join(destination, "receipt.json"), "utf8")) !== receipt) {
                    throw new UpdateError(
                        "INSTALL",
                        "installed release directory contains different content",
                    );
                }
                await check(destination);
            }

            return { release: download.release, sha256: download.sha256, directory: destination };
        } finally {
            await rm(staging, { recursive: true, force: true });
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
            throw new UpdateError(
                "INSTALL",
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
        const temporary = join(this.directory, `activate.${crypto.randomUUID()}.json`);
        await writeFile(temporary, JSON.stringify(record), { flag: "wx", mode: 0o600 });
        await rename(temporary, join(this.directory, "activate.json"));
        try {
            await this.recover();
        } catch (error) {
            throw new UpdateError(
                "ACTIVATION",
                "activation did not finish; retry `destack self activate` before restarting applications",
                { cause: error },
            );
        }
    }

    /** Complete a recorded activation after affected processes have stopped. */
    async recover(): Promise<void> {
        // read only activations that were committed to the installation directory
        const source = await readOptional(join(this.directory, "activate.json"));
        if (source === undefined) {
            return;
        }

        // verify the staged receipt before replacing the native application
        const parsed = ActivationRecord.safeParse(JSON.parse(source));
        if (!parsed.success) {
            throw new UpdateError("INSTALL", "invalid activation record");
        }
        const record = parsed.data;
        const release = new Release(record.version, record.target);
        const directory = this.path(release);
        const receipt = Receipt.safeParse(
            JSON.parse(await readFile(join(directory, "receipt.json"), "utf8")),
        );
        if (!receipt.success || receipt.data.sha256 !== record.sha256) {
            throw new UpdateError(
                "INSTALL",
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
                throw new UpdateError(
                    "INSTALL",
                    "missing absolute application destination or identifier",
                );
            }
            await installApplication(
                join(directory, "Destack.app"),
                application,
                applicationIdentifier,
            );
        }

        // select the release only after platform installation succeeds
        const current =
            JSON.stringify({
                version: release.version,
                target: release.target,
                sha256: record.sha256,
            }) + "\n";
        const pending = join(this.directory, `current.${crypto.randomUUID()}.json`);
        await writeFile(pending, current, { flag: "wx", mode: 0o600 });
        await rename(pending, join(this.directory, "current.json"));
        await rm(join(this.directory, "staged.json"), { force: true });
        await rm(join(this.directory, "activate.json"));
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
            throw new UpdateError(
                "INSTALL",
                "refusing to activate an older or incompatible release",
            );
        }
        if (current && current.release.compare(release) === 0 && current.sha256 !== sha256) {
            throw new UpdateError("INSTALL", "published release changed");
        }
    }
}

/** Reject symlinks that escape the extracted distribution. */
async function verifyLinks(root: string, directory: string): Promise<void> {
    for (const name of await readdir(directory)) {
        const path = join(directory, name);
        const stat = await lstat(path);
        if (stat.isSymbolicLink()) {
            const link = await readlink(path);
            const destination = resolve(directory, link);
            const relation = relative(root, destination);
            if (isAbsolute(link) || relation === ".." || relation.startsWith(`..${sep}`)) {
                throw new UpdateError("INSTALL", `archive link escapes the distribution: ${path}`);
            }
        } else if (stat.isDirectory()) {
            await verifyLinks(root, path);
        }
    }
}

/** Find why an archive entry could write outside the staged distribution, if it could. */
function rejectEntry(
    path: string,
    entry: Stats | ReadEntry,
    root: string,
): UpdateError | undefined {
    // split the path on both separators
    const components = path.replaceAll("\\", "/").split("/");

    // require a tar entry
    if (!("type" in entry)) {
        return new UpdateError("INSTALL", "expected an archive entry");
    }
    // require a relative path inside the distribution
    else if (isAbsolute(path) || components.includes("..") || /^[A-Za-z]:/u.test(path)) {
        return new UpdateError("INSTALL", `unsafe archive path: ${path}`);
    }
    // require a supported entry type
    else if (!["File", "Directory", "SymbolicLink"].includes(entry.type)) {
        return new UpdateError("INSTALL", `unsupported archive entry: ${path} (${entry.type})`);
    }
    // accept files and directories
    else if (entry.type !== "SymbolicLink") {
        return undefined;
    }

    // require a link target inside the distribution
    if (entry.linkpath === undefined || entry.linkpath === "") {
        return new UpdateError("INSTALL", `missing archive link target: ${path}`);
    }
    const target = resolve(root, path, "..", entry.linkpath);
    const relation = relative(root, target);
    if (isAbsolute(entry.linkpath) || relation === ".." || relation.startsWith(`..${sep}`)) {
        return new UpdateError("INSTALL", `archive link escapes the distribution: ${path}`);
    }

    return undefined;
}

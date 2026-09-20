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
import { installApplication } from "./macos.ts";

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
        const record = { ...staged.release, sha256: staged.sha256, previous: staged.previous };
        const temporary = join(this.directory, `staged.${crypto.randomUUID()}.json`);
        await writeFile(temporary, JSON.stringify(record), { flag: "wx", mode: 0o600 });
        await rename(temporary, join(this.directory, "staged.json"));
    }

    /** Read the distribution waiting for an explicit restart. */
    async staged(): Promise<StagedRelease | undefined> {
        let source: string;
        try {
            source = await readFile(join(this.directory, "staged.json"), "utf8");
        } catch (error) {
            if ((error as NodeJS.ErrnoException).code === "ENOENT") return undefined;
            throw error;
        }

        // validate persisted identifiers before constructing installation paths
        const record = JSON.parse(source);
        const release = new Release(record.version, record.target);
        if (
            typeof record.sha256 !== "string" || !/^[0-9a-f]{64}$/.test(record.sha256) ||
            (record.previous !== undefined && (typeof record.previous !== "string" ||
                !/^[0-9a-f]{64}$/.test(record.previous)))
        ) {
            throw new UpdateError("INSTALL", "Invalid staged release digest.");
        }

        return {
            release,
            sha256: record.sha256,
            previous: record.previous,
            directory: this.path(release),
        };
    }

    /** Read the current release without modifying application files. */
    async current(): Promise<InstalledRelease | undefined> {
        let source: string;
        try {
            source = await readFile(join(this.directory, "current.json"), "utf8");
        } catch (error) {
            if ((error as NodeJS.ErrnoException).code === "ENOENT") return undefined;
            throw error;
        }
        const record = JSON.parse(source);
        const release = new Release(record.version, record.target);
        if (typeof record.sha256 !== "string" || !/^[0-9a-f]{64}$/.test(record.sha256)) {
            throw new UpdateError("INSTALL", "Invalid installed release digest.");
        }

        return { release, sha256: record.sha256, directory: this.path(release) };
    }

    /** Extract a verified distribution and check its executables before activation. */
    async stage(
        download: Download,
        check: (directory: string) => Promise<void>,
    ): Promise<InstalledRelease> {
        // reject downgrades and reuse only an identical authenticated release
        const current = await this.current();
        if (current && current.release.target !== download.release.target) {
            throw new UpdateError("INSTALL", "Cannot change the installed platform.");
        }
        if (current && download.release.compare(current.release) < 0) {
            throw new UpdateError("INSTALL", "Refusing to install an older release.");
        }
        if (current && download.release.compare(current.release) === 0) {
            if (current.sha256 !== download.sha256) {
                throw new UpdateError("INSTALL", "Published release changed.");
            }
            await check(current.directory);
            return current;
        }

        // extract into a fresh sibling directory without touching the active distribution
        const versions = join(this.directory, "versions");
        await mkdir(versions, { recursive: true, mode: 0o700 });
        const staging = await mkdtemp(join(versions, ".install-"));
        try {
            let rejected: unknown;
            await extract({
                file: download.archive,
                cwd: staging,
                strict: true,
                preservePaths: false,
                filter(path, entry) {
                    if (rejected) return false;
                    try {
                        verifyEntry(path, entry, staging);
                        return true;
                    } catch (error) {
                        // reject through the extraction promise after the parser drains
                        rejected = error;
                        return false;
                    }
                },
            });
            if (rejected) throw rejected;
            await verifyLinks(staging, staging);
            await check(staging);

            // retain verified releases after interrupted activation without overwriting them
            const destination = this.path(download.release);
            const receipt = JSON.stringify({ sha256: download.sha256 });
            await writeFile(join(staging, "receipt.json"), receipt, { flag: "wx", mode: 0o600 });
            try {
                await rename(staging, destination);
            } catch (error) {
                const code = (error as NodeJS.ErrnoException).code;
                if (code !== "EEXIST" && code !== "ENOTEMPTY") throw error;
                if (await readFile(join(destination, "receipt.json"), "utf8") !== receipt) {
                    throw new UpdateError(
                        "INSTALL",
                        "Installed release directory contains different content.",
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
    async activate(installed: InstalledRelease, application?: string): Promise<void> {
        // reject stale activation before recording its intent
        await this.checkActivation(installed.release, installed.sha256);

        // require a destination for native macOS bundles
        const isMac = installed.release.target.endsWith("apple-darwin");
        if (isMac !== (application !== undefined)) {
            throw new UpdateError("INSTALL", "macOS releases require an application destination.");
        }

        // persist the intended release before changing either active location
        const record = {
            version: installed.release.version,
            target: installed.release.target,
            sha256: installed.sha256,
            application,
        };
        const temporary = join(this.directory, `activate.${crypto.randomUUID()}.json`);
        await writeFile(temporary, JSON.stringify(record), { flag: "wx", mode: 0o600 });
        await rename(temporary, join(this.directory, "activate.json"));
        try {
            await this.recover();
        } catch (error) {
            throw new UpdateError(
                "ACTIVATION",
                "Activation did not finish. Retry destack update --activate before restarting applications.",
                { cause: error },
            );
        }
    }

    /** Complete a recorded activation after affected processes have stopped. */
    async recover(): Promise<void> {
        // read only activations that were committed to the installation directory
        let source: string;
        try {
            source = await readFile(join(this.directory, "activate.json"), "utf8");
        } catch (error) {
            if ((error as NodeJS.ErrnoException).code === "ENOENT") return;
            throw error;
        }

        // verify the staged receipt before replacing the native application
        const record = JSON.parse(source);
        const release = new Release(record.version, record.target);
        const directory = this.path(release);
        const receipt = JSON.parse(await readFile(join(directory, "receipt.json"), "utf8"));
        if (
            typeof record.sha256 !== "string" || !/^[0-9a-f]{64}$/.test(record.sha256) ||
            receipt.sha256 !== record.sha256
        ) {
            throw new UpdateError(
                "INSTALL",
                "Activation receipt does not match the staged release.",
            );
        }
        await this.checkActivation(release, record.sha256);
        if (release.target.endsWith("apple-darwin")) {
            if (typeof record.application !== "string" || !isAbsolute(record.application)) {
                throw new UpdateError("INSTALL", "Missing absolute application destination.");
            }
            await installApplication(join(directory, "Destack.app"), record.application);
        }

        // select the release only after platform installation succeeds
        const current = JSON.stringify({
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
            current && (current.release.target !== release.target ||
                current.release.compare(release) > 0)
        ) {
            throw new UpdateError(
                "INSTALL",
                "Refusing to activate an older or incompatible release.",
            );
        }
        if (current && current.release.compare(release) === 0 && current.sha256 !== sha256) {
            throw new UpdateError("INSTALL", "Published release changed.");
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
                throw new UpdateError("INSTALL", `Archive link escapes the distribution: ${path}`);
            }
        } else if (stat.isDirectory()) {
            await verifyLinks(root, path);
        }
    }
}

/** Reject archive paths and entry types that can write outside the staged distribution. */
function verifyEntry(path: string, entry: Stats | ReadEntry, root: string): void {
    if (!("type" in entry)) {
        throw new UpdateError("INSTALL", "Expected an archive entry.");
    }
    const components = path.replaceAll("\\", "/").split("/");
    if (
        isAbsolute(path) || components.includes("..") || /^[A-Za-z]:/.test(path)
    ) {
        throw new UpdateError("INSTALL", `Unsafe archive path: ${path}`);
    }
    if (!["File", "Directory", "SymbolicLink"].includes(entry.type)) {
        throw new UpdateError(
            "INSTALL",
            `Unsupported archive entry: ${path} (${entry.type})`,
        );
    }
    if (entry.type === "SymbolicLink") {
        if (!entry.linkpath) {
            throw new UpdateError(
                "INSTALL",
                `Missing archive link target: ${path}`,
            );
        }
        const target = resolve(root, path, "..", entry.linkpath);
        const relation = relative(root, target);
        if (
            isAbsolute(entry.linkpath) || relation === ".." ||
            relation.startsWith(`..${sep}`)
        ) {
            throw new UpdateError(
                "INSTALL",
                `Archive link escapes the distribution: ${path}`,
            );
        }
    }
}

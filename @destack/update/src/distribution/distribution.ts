import { dlopen } from "bun:ffi";
import { type Environment, FileSystemError, type Process } from "@destack/environment";
import { schema, Uuid } from "@destack/schema";
import { verifyApplication } from "../apple/install.ts";
import { ShellPath } from "../install/path.ts";
import { basename, dirname, isAbsolute, join, resolve } from "node:path";
import type { StagedRelease } from "../install/installer.ts";
import {
    MACHINE_EXECUTABLES,
    RELEASE_ROOT,
    Release,
    type ReleaseChannel,
} from "../release/index.ts";
import { UpdateError } from "../error/index.ts";
import { Updater } from "../update/index.ts";

/** The permissions of the installation directory and its launchers: the person's alone. */
const PRIVATE_MODE = 0o700;

/** The permissions of a desktop entry, which the desktop environment reads (XDG Desktop Entry). */
const ENTRY_MODE = 0o644;

/** The size of darwin's `struct statvfs`. */
const STATVFS_SIZE = 64;

/** The byte offset of `f_flag` in darwin's `struct statvfs`. */
const STATVFS_FLAG_OFFSET = 48;

/** The `ST_RDONLY` bit of `f_flag`, from darwin's `<sys/statvfs.h>`. */
const ST_RDONLY = 1n;

/** The command's executable inside a native Windows package's application directory. */
const WINDOWS_COMMAND = "helpers/destack.exe";

/** The system libraries the native Linux desktop needs. */
const DESKTOP_LIBRARIES = ["libgtk-3.so.0", "libwebkit2gtk-4.1.so.0"] as const;

/** The installer responsible for a distribution's files and their removal. */
export const DistributionMethod = schema.enum(["archive", "application", "nsis"]);
/** The installer responsible for a distribution's files and their removal. */
export type DistributionMethod = schema.Infer<typeof DistributionMethod>;

/** How a distribution was installed: its installer and its application. */
export const DistributionRecord = schema.object({
    /** Installer responsible for this application. */
    method: DistributionMethod,
    /** Absolute path to the installed application bundle or directory. */
    application: schema.string(),
});
/** How a distribution was installed: its installer and its application. */
export type DistributionRecord = schema.Infer<typeof DistributionRecord>;

/** The native package descriptor an installer writes beside the application. */
const NativeDescriptor = schema.looseObject({
    /** The installer that wrote it. */
    method: schema.string(),
    /** The installed release's version. */
    version: schema.string(),
    /** The installed release's target. */
    target: schema.string(),
});

/** The effects a distribution runs with: the files, the process and the host computer. */
export type DistributionEnvironment = Environment<"files" | "process" | "host">;

/** What a distribution is: its location, followed repository and running release. */
export interface DistributionOptions {
    /** The home directory, which keeps the distribution files and their registration. */
    readonly home: string;
    /** The signed repository the releases come from. */
    readonly repository: URL;
    /** The root metadata the repository is trusted from. */
    readonly root: string;
    /** The release this executable runs. */
    readonly running: Release;
    /** This executable, inside the Mac application when one bundles it. */
    readonly executable: string;
    /** The directory Mac applications install into. */
    readonly applications: string;
    /** The release channel installed, which names its commands, application and identifier. */
    readonly channel: ReleaseChannel;
    /** The clock, network, files and processes updates run on. */
    readonly environment: DistributionEnvironment;
}

/** This machine's installed Destack distribution. */
export class Distribution {
    /** What the distribution is. */
    readonly options: DistributionOptions;

    /** Describe an installed distribution. */
    constructor(options: DistributionOptions) {
        this.options = options;
    }

    /** The directory with the versioned releases and their activation records. */
    get directory(): string {
        return join(this.options.home, "distribution");
    }

    /** Find the Mac application the distribution installs. */
    async application(): Promise<string | undefined> {
        // leave other platforms without a Mac application
        if (!this.options.running.target.endsWith("apple-darwin")) {
            return undefined;
        }
        const registration = await this.record();

        return (
            registration?.application ??
            Distribution.bundle(this.options.executable) ??
            join(this.options.applications, `${this.options.channel.title}.app`)
        );
    }

    /** Read how the distribution was installed, absent before installation. */
    async record(): Promise<DistributionRecord | undefined> {
        // read the atomic registration document, absent before installation
        const { files } = this.options.environment;
        const path = join(this.options.home, "distribution.json");
        const text = await files.readTextFile(path).catch(FileSystemError.absent);

        return text === undefined ? undefined : this.#validate(JSON.parse(text));
    }

    /** Record a verified application without changing its installer ownership. */
    async register(record: DistributionRecord): Promise<void> {
        // reject implicit conversion between independently managed installations
        this.#validate(record);
        const current = await this.record();
        if (current !== undefined && current.method !== record.method) {
            throw UpdateError.of(
                "INSTALLATION_METHOD_MISMATCH",
                "uninstall the existing installation before changing its method",
            );
        }

        // publish the complete registration in one rename
        const { files, random } = this.options.environment;
        await files.mkdir(this.options.home, { recursive: true, mode: PRIVATE_MODE });
        const pending = join(this.options.home, `distribution.${Uuid.v4(random)}.json`);
        await files.writeTextFile(pending, JSON.stringify(record) + "\n", {
            createNew: true,
            mode: 0o600,
        });
        await files.rename(pending, join(this.options.home, "distribution.json"));
    }

    /** Register an installed application and link its command, answering its machine. */
    async link(
        record: DistributionRecord,
        options: {
            /** Whether to add the command to the user's shell and menus. */
            readonly isShell: boolean;
        },
    ): Promise<string | undefined> {
        // leave a mounted or translocated Mac application unregistered
        const { application, method } = record;
        const { channel } = this.options;
        const { files, host } = this.options.environment;
        if (method === "application" && Distribution.#isTransient(application)) {
            return undefined;
        }

        // verify a Mac application, then record the installation
        if (method === "application") {
            await verifyApplication(
                application,
                channel.applicationIdentifier,
                this.options.environment,
            );
        }
        await this.register(record);

        // link the command into the installation's bin, by the installer's own kind of launcher
        const bin = join(this.options.home, "bin");
        await files.mkdir(bin, { recursive: true, mode: PRIVATE_MODE });
        const executable =
            method === "application"
                ? await this.#linkApplication(application, bin, channel)
                : method === "nsis"
                  ? await this.#linkNsis(bin, channel)
                  : await this.#linkArchive(application, bin, channel);

        // add the command to the user's shell and, on Linux, the archive's desktop to the menus
        if (options.isShell) {
            await new ShellPath(bin, channel, this.options.environment).add();
        }
        if (options.isShell && method === "archive" && host.os === "linux") {
            await this.#addDesktopEntry(channel, join(bin, channel.command), application);
        }

        return executable;
    }

    /** Remove the channel's launcher from the user's shell and menus, keeping the files. */
    async unlink(options: { readonly isShell: boolean }): Promise<void> {
        // remove the exact PATH entries, and the Linux desktop entry and icon
        const { channel } = this.options;
        if (options.isShell) {
            await new ShellPath(
                join(this.options.home, "bin"),
                channel,
                this.options.environment,
            ).remove();
        }
        if (options.isShell && this.options.environment.host.os === "linux") {
            const directory = this.#desktopDirectory();
            const { files } = this.options.environment;
            const identifier = channel.applicationIdentifier;
            await files.remove(join(directory, "applications", `${identifier}.desktop`), {
                force: true,
            });
            await files.remove(
                join(directory, "icons/hicolor/scalable/apps", `${identifier}.svg`),
                {
                    force: true,
                },
            );
        }
    }

    /** Read the native Windows package that installed this executable, absent elsewhere. */
    static async native(
        environment: DistributionEnvironment,
    ): Promise<DistributionRecord | undefined> {
        // read nothing outside Windows
        const { files, host } = environment;
        if (host.os !== "win32") {
            return undefined;
        }

        // locate the package descriptor beside the application containing this executable
        const executable = await files.realPath(environment.process.executable);
        const application = dirname(dirname(executable));
        const source = await files
            .readTextFile(join(application, "installation.json"))
            .catch(FileSystemError.absent);
        if (source === undefined) {
            return undefined;
        }

        // accept only a descriptor matching this compiled package, its layout and a command path
        const parsed = NativeDescriptor.safeParse(JSON.parse(source));
        const release = new Release(
            import.meta.destack.package.version,
            Release.target(host.os, host.architecture),
        );
        const command = await files.realPath(join(application, WINDOWS_COMMAND));
        const isMatching =
            parsed.success &&
            parsed.data.method === "nsis" &&
            parsed.data.version === release.version &&
            parsed.data.target === release.target &&
            executable === command;
        if (!isMatching) {
            throw UpdateError.of(
                "INSTALLATION_MISMATCH",
                "native installation descriptor does not match this executable",
            );
        } else if (/[\r\n"]/u.test(executable)) {
            throw UpdateError.of(
                "INSTALLATION_MISMATCH",
                "native installation path contains unsupported command characters",
            );
        }

        return { method: "nsis", application };
    }

    /** Require removal of a native package to run from that package's own executable. */
    async requireNative(record: DistributionRecord | undefined): Promise<void> {
        // require this executable to be a native package's
        if ((await Distribution.native(this.options.environment)) === undefined) {
            throw UpdateError.of(
                "INSTALLATION_MISMATCH",
                "native removal requires the Windows package executable",
            );
        }

        // require the registered installation to be this native package
        const { files, process } = this.options.environment;
        const command =
            record === undefined ? undefined : join(record.application, WINDOWS_COMMAND);
        if (record !== undefined && record.method !== "nsis") {
            throw UpdateError.of(
                "INSTALLATION_MISMATCH",
                "native removal cannot remove a different installation method",
            );
        } else if (
            command !== undefined &&
            (await files.realPath(process.executable)) !== (await files.realPath(command))
        ) {
            throw UpdateError.of(
                "INSTALLATION_MISMATCH",
                "native removal must run from the registered installation",
            );
        }
    }

    /** Require the system libraries the native Linux desktop needs, and a regular user. */
    static async requireDesktop(process: Process): Promise<void> {
        // refuse privileged installation, leaving dependency installation to the person
        if (process.userId === 0) {
            throw UpdateError.of(
                "INSTALLATION_MISMATCH",
                "install Destack as your regular user, without sudo",
            );
        }

        // refuse a missing library, naming the command that installs it
        for (const library of DESKTOP_LIBRARIES) {
            if (!(await process.hasLibrary(library))) {
                throw UpdateError.of(
                    "DESKTOP_UNAVAILABLE",
                    `desktop dependency ${library} is unavailable; run ${Distribution.#dependencyCommand(process)}`,
                );
            }
        }
    }

    /** Locate a helper executable inside a Mac application. */
    static helper(application: string, name: string): string {
        return join(application, "Contents", "Helpers", name);
    }

    /** Parse a record, rejecting unknown installers and relative application locations. */
    #validate(value: unknown): DistributionRecord {
        // validate persisted registration before using it for installation operations
        const record = DistributionRecord.safeParse(value);
        if (!record.success || !isAbsolute(record.data.application)) {
            throw UpdateError.of("RECORD_INVALID", "invalid application registration");
        }

        return record.data;
    }

    /** Link a Mac application's command into bin, replacing another link atomically. */
    async #linkApplication(application: string, bin: string, channel: ReleaseChannel) {
        // link the bundle's command unless the link already points at it
        const { files, random } = this.options.environment;
        const path = join(bin, channel.command);
        const target = Distribution.helper(application, "destack");
        if ((await files.readLink(path).catch(FileSystemError.absent)) !== target) {
            const pending = join(bin, `.destack-${Uuid.v4(random)}`);
            await files.symlink(target, pending);
            await files.rename(pending, path);
        }

        return Distribution.helper(application, MACHINE_EXECUTABLES.darwin);
    }

    /** Write a native package's command shim into bin, following the activated Windows release. */
    async #linkNsis(bin: string, channel: ReleaseChannel) {
        // write a shim calling this executable, renamed into place
        const { files, process, random } = this.options.environment;
        const executable = await files.realPath(process.executable);
        const pending = join(bin, `.${channel.command}-${Uuid.v4(random)}.cmd`);
        const escaped = executable.replaceAll("%", "%%");
        await files.writeTextFile(
            pending,
            `@echo off\r\nsetlocal DisableDelayedExpansion\r\n"${escaped}" %*\r\n`,
            { createNew: true },
        );
        await files.rename(pending, join(bin, `${channel.command}.cmd`));

        return executable;
    }

    /** Link an activated archive's command into bin, keeping a launcher already there. */
    async #linkArchive(application: string, bin: string, channel: ReleaseChannel) {
        // hard link the release's command, keeping an existing launcher
        const { files, host } = this.options.environment;
        const release = dirname(application);
        await files
            .link(join(release, "bin", "destack"), join(bin, channel.command))
            .catch((error: unknown) => {
                if (!FileSystemError.is(error, "ALREADY_EXISTS")) {
                    throw error;
                }
            });

        return join(release, "bin", MACHINE_EXECUTABLES[host.os]);
    }

    /** Register the desktop in the user's menu and as its channel's link handler. */
    async #addDesktopEntry(channel: ReleaseChannel, launcher: string, application: string) {
        // require a launcher path a desktop entry accepts
        if (!isAbsolute(launcher) || /[\r\n]/u.test(launcher)) {
            throw UpdateError.of(
                "ENVIRONMENT_INVALID",
                "desktop launcher must be an absolute single-line path",
            );
        }

        // copy the icon under the XDG data directory
        const { files, random } = this.options.environment;
        const directory = this.#desktopDirectory();
        const identifier = channel.applicationIdentifier;
        const icons = join(directory, "icons/hicolor/scalable/apps");
        const applications = join(directory, "applications");
        await files.mkdir(icons, { recursive: true });
        await files.mkdir(applications, { recursive: true });
        await files.copyFile(join(application, "icon.svg"), join(icons, `${identifier}.svg`));

        // escape for the Exec field's quoting and then for the desktop-entry string value
        const command = launcher
            .replaceAll("\\", "\\\\")
            .replaceAll('"', '\\"')
            .replaceAll("`", "\\`")
            .replaceAll("$", "\\$")
            .replaceAll("%", "%%")
            .replaceAll("\\", "\\\\");

        // write the entry atomically, routing the channel's link scheme to the desktop
        const source = `[Desktop Entry]\nType=Application\nName=${channel.title}\nExec="${command}" desktop %u\nIcon=${identifier}\nTerminal=false\nCategories=Utility;Development;\nMimeType=x-scheme-handler/${channel.command};\n`;
        const pending = join(applications, `.${identifier}-${Uuid.v4(random)}`);
        await files.writeTextFile(pending, source, { mode: ENTRY_MODE, createNew: true });
        await files.rename(pending, join(applications, `${identifier}.desktop`));
    }

    /** Resolve the per-user desktop integration directory, as XDG sets it. */
    #desktopDirectory(): string {
        // take XDG_DATA_HOME, else the default under the home, refusing a relative or multi-line one
        const { process, host } = this.options.environment;
        const directory = process.variables["XDG_DATA_HOME"] ?? join(host.home, ".local/share");
        if (!isAbsolute(directory) || /[\r\n]/u.test(directory)) {
            throw UpdateError.of(
                "ENVIRONMENT_INVALID",
                "XDG_DATA_HOME must be an absolute single-line path",
            );
        }

        return directory;
    }

    /** Name the command installing the desktop's libraries with this system's package manager. */
    static #dependencyCommand(process: Process): string {
        // install through APT
        if (process.which("apt-get") !== undefined) {
            return "sudo apt-get install libwebkit2gtk-4.1-0";
        }
        // install through DNF
        else if (process.which("dnf") !== undefined) {
            return "sudo dnf install webkit2gtk4.1 gtk3";
        }
        // list the libraries for any other package manager
        else {
            return "install GTK 3 and WebKitGTK 4.1 using your distribution's package manager";
        }
    }

    /** Report whether a Mac application runs from a read-only volume or a translocated download. */
    static #isTransient(application: string): boolean {
        // treat a translocated download as transient without reading its volume
        if (application.includes("/AppTranslocation/")) {
            return true;
        }

        // read the volume's status into darwin's statvfs layout
        const system = dlopen("/usr/lib/libSystem.B.dylib", {
            statvfs: { args: ["buffer", "buffer"], returns: "i32" },
        });
        try {
            const status = new Uint8Array(STATVFS_SIZE);
            if (
                system.symbols.statvfs(new TextEncoder().encode(application + "\0"), status) !== 0
            ) {
                throw UpdateError.of(
                    "FILE_SYSTEM_FAILED",
                    `cannot read the filesystem status of ${application}`,
                );
            }
            const flag = new DataView(status.buffer).getBigUint64(STATVFS_FLAG_OFFSET, true);

            return (flag & ST_RDONLY) !== 0n;
        } finally {
            system.close();
        }
    }

    /** Read the root metadata every release bundles, which the feed is trusted from. */
    static root(): string {
        return JSON.stringify(RELEASE_ROOT);
    }

    /** Find the Mac application containing an executable, absent outside one. */
    static bundle(executable: string): string | undefined {
        let directory = dirname(resolve(executable));
        while (dirname(directory) !== directory) {
            if (basename(directory).endsWith(".app")) {
                return directory;
            }
            directory = dirname(directory);
        }

        return undefined;
    }

    /** Open a locked update session, refusing an installation that changed under it. */
    async open(isInstalling = false): Promise<Updater> {
        // refuse a development build, which takes no published updates
        if (this.options.channel.name === "dev") {
            throw UpdateError.of(
                "DEVELOPMENT_BUILD",
                "development builds cannot take published updates",
            );
        }

        // read how the distribution was installed, and where its Mac application lives
        const registration = await this.record();
        const application = await this.application();
        const { running } = this.options;
        const updater = await Updater.open({
            directory: this.directory,
            repository: this.options.repository,
            root: this.options.root,
            target: running.target,
            ...(isInstalling ? {} : { current: running }),
            ...(application === undefined ? {} : { application }),
            applicationIdentifier: this.options.channel.applicationIdentifier,
            environment: this.options.environment,
        });

        // refuse an installation another installer changed before the lock was taken
        try {
            const locked = await this.record();
            if (JSON.stringify(locked) !== JSON.stringify(registration)) {
                throw UpdateError.of(
                    "INSTALLATION_CHANGED",
                    "installation changed; retry the operation",
                );
            } else if (running.target.includes("windows") && registration?.method !== "nsis") {
                throw UpdateError.of(
                    "INSTALLATION_METHOD_MISMATCH",
                    "install Destack using Setup.exe before updating",
                );
            }

            return updater;
        } catch (error) {
            await updater[Symbol.asyncDispose]();
            throw error;
        }
    }

    /** Download and stage the latest release when it is newer than the installed one. */
    async stage(archive?: string): Promise<StagedRelease | undefined> {
        // keep a release staged earlier until a newer one arrives
        await using updater = await this.open();
        const available = await updater.check();
        if (available === undefined) {
            return updater.staged();
        }
        const download = await available.download(archive === undefined ? {} : { archive });

        return updater.stage(download);
    }

    /** Read the installed release, the running one before the first activation. */
    async installed(): Promise<Release> {
        // take the recorded release once it is newer than the running one
        await using updater = await this.open();
        const recorded = (await updater.current())?.release;
        const { running } = this.options;

        return recorded !== undefined && recorded.compare(running) > 0 ? recorded : running;
    }

    /** Read the release staged for activation, absent when none is. */
    async staged(): Promise<StagedRelease | undefined> {
        await using updater = await this.open();

        return updater.staged();
    }
}

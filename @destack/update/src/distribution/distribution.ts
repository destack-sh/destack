import { basename, dirname, join, resolve } from "node:path";
import { Installation } from "../installation/index.ts";
import type { StagedRelease } from "../install/installer.ts";
import type { Channel, Release } from "../release/index.ts";
import { UpdateError } from "../error/index.ts";
import { Updater } from "../update/index.ts";
import nightly from "./root/nightly.json" with { type: "json" };
import stable from "./root/stable.json" with { type: "json" };

/** The root metadata each channel's repository is trusted from, bundled with every release. */
const ROOTS = { stable, nightly } as const satisfies Record<Channel, unknown>;

/** What a distribution is: its channel, where it lives, the repository it follows and the release running. */
export interface DistributionOptions {
    /** The channel the distribution follows. */
    readonly channel: Channel;
    /** The channel's home directory, which keeps the distribution files and their registration. */
    readonly home: string;
    /** The signed repository the channel's releases come from. */
    readonly repository: URL;
    /** The root metadata the repository is trusted from, the channel's bundled root in a release. */
    readonly root: string;
    /** The release this executable runs. */
    readonly running: Release;
    /** This executable, inside the Mac application when one bundles it. */
    readonly executable: string;
    /** The directory Mac applications install into. */
    readonly applications: string;
    /** The Mac application's bundle identifier. */
    readonly applicationIdentifier: string;
    /** The application's display name, such as `Destack Nightly`. */
    readonly title: string;
}

/** This machine's installed Destack distribution: the releases it stages and activates from its channel's signed repository. */
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

    /** Find the Mac application the distribution installs: the registered one, the one running this executable, or the channel's own. */
    async application(): Promise<string | undefined> {
        // leave other platforms, which install no Mac application
        if (!this.options.running.target.endsWith("apple-darwin")) {
            return undefined;
        }
        const registration = await new Installation(dirname(this.directory)).read();

        return (
            registration?.application ??
            Distribution.bundle(this.options.executable) ??
            join(this.options.applications, `${this.options.title}.app`)
        );
    }

    /** Read the root metadata a channel's releases bundle, which its repository is trusted from. */
    static root(channel: Channel): string {
        return JSON.stringify(ROOTS[channel]);
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

    /** Open a locked update session, a first installation's without a running release, refusing an installation that changed under it. */
    async open(isInstalling = false): Promise<Updater> {
        // read how the distribution was installed, and where its Mac application lives
        const installation = new Installation(dirname(this.directory));
        const registration = await installation.read();
        const application = await this.application();
        const { running } = this.options;
        const updater = await Updater.open({
            directory: this.directory,
            repository: this.options.repository,
            channel: this.options.channel,
            root: this.options.root,
            target: running.target,
            ...(isInstalling ? {} : { current: running }),
            ...(application === undefined ? {} : { application }),
            applicationIdentifier: this.options.applicationIdentifier,
        });

        // refuse an installation another installer changed before the lock was taken
        try {
            const locked = await installation.read();
            if (JSON.stringify(locked) !== JSON.stringify(registration)) {
                throw new UpdateError("INSTALL", "installation changed; retry the operation");
            } else if (running.target.includes("windows") && registration?.method !== "nsis") {
                throw new UpdateError("INSTALL", "install Destack using Setup.exe before updating");
            }

            return updater;
        } catch (error) {
            await updater[Symbol.asyncDispose]();
            throw error;
        }
    }

    /** Download and stage the latest release when it is newer than the installed one, returning the staged release, none when it is current. */
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

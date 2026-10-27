import type { Environment, OperatingSystem } from "@destack/environment";
import { UpdateError } from "../error/error.ts";

/** The names of the channels a build selects. */
const NAMES = ["stable", "dev"] as const;

/** The distribution tag of stable releases, npm's default. */
const STABLE_TAG = "latest";

/** The update feed every published release comes from, the `stable` path of the download origin. */
const FEED = "https://download.destack.sh/stable/";

/** The account service each channel signs people in with. */
const ISSUERS = {
    stable: "https://destack.app",
    dev: "http://127.0.0.1:4100",
} as const satisfies Readonly<Record<(typeof NAMES)[number], string>>;

/** The machine executable's file name a release ships on each operating system. */
export const MACHINE_EXECUTABLES: Readonly<Record<OperatingSystem, string>> = {
    darwin: "destack-machine",
    linux: "destack-machine",
    win32: "destack-machine.exe",
};

/** The distribution a build selected: its command, directory name and application identity. */
export class ReleaseChannel {
    /** The names of the channels a build selects. */
    static readonly names = NAMES;

    /** The channel's name. */
    readonly name: (typeof NAMES)[number];

    /** Select a channel by its name. */
    constructor(name: (typeof NAMES)[number]) {
        this.name = name;
    }

    /** Read a channel by its name, refusing any other. */
    static of(name: string | undefined): ReleaseChannel {
        if (!isName(name)) {
            throw UpdateError.of("CHANNEL_UNKNOWN", `unknown release channel ${name}`);
        }

        return new ReleaseChannel(name);
    }

    /** Read the channel the build selected for this executable, from its environment. */
    static current(environment: Environment<"process">): ReleaseChannel {
        return ReleaseChannel.of(environment.process.variables["DESTACK_RELEASE_CHANNEL"]);
    }

    /** The account service the channel signs people in with. */
    get issuer(): string {
        return ISSUERS[this.name];
    }

    /** The distribution tag of the first-party releases the channel installs. */
    get tag(): string {
        return this.name === "stable" ? STABLE_TAG : this.name;
    }

    /** The update feed the channel's installations follow. */
    get feed(): URL {
        return new URL(FEED);
    }

    /** The suffix of the channel's commands, directories and artwork, empty for stable. */
    get suffix(): string {
        return this.name === "stable" ? "" : `-${this.name}`;
    }

    /** The native application identifier, such as `app.destack.desktop.dev`. */
    get applicationIdentifier(): string {
        return `app.destack.desktop${this.name === "stable" ? "" : `.${this.name}`}`;
    }

    /** The application's display name, such as `Destack Dev`. */
    get title(): string {
        return this.name === "stable"
            ? "Destack"
            : `Destack ${this.name.charAt(0).toUpperCase()}${this.name.slice(1)}`;
    }

    /** The command the channel installs, such as `destack-dev`. */
    get command(): string {
        return `destack${this.suffix}`;
    }

    /** The channel's device directory name in a user's home directory, such as `.destack-dev`. */
    get directory(): string {
        return `.destack${this.suffix}`;
    }
}

/** Check whether a name is a release channel's. */
function isName(name: string | undefined): name is (typeof NAMES)[number] {
    return NAMES.some((candidate) => candidate === name);
}

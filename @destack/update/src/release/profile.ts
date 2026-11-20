import type { Environment } from "@destack/environment";
import { UpdateError } from "../error/error.ts";

/** The names of the profiles a build is: a published release, or a checkout's local build. */
const NAMES = ["release", "local"] as const;

/** The universe origin each profile signs people in at by default. */
const UNIVERSES = {
    release: "https://destack.app",
    local: "http://destack.localhost:4100",
} as const satisfies Readonly<Record<(typeof NAMES)[number], string>>;

/** The profile an executable is built as: its command, identity, directory and universe. */
export class Profile {
    /** The names of the profiles an executable is built as. */
    static readonly names = NAMES;

    /** The profile's name. */
    readonly name: (typeof NAMES)[number];

    /** Select a profile by its name. */
    constructor(name: (typeof NAMES)[number]) {
        this.name = name;
    }

    /** Read a profile by its name, refusing any other. */
    static of(name: string | undefined): Profile {
        const found = NAMES.find((candidate) => candidate === name);
        if (found === undefined) {
            throw UpdateError.of("PROFILE_INVALID", `profile '${name}' is invalid: it is unknown`);
        }

        return new Profile(found);
    }

    /** Read the profile the build selected from the process's variables. */
    static current(environment: Environment<"process">): Profile {
        return Profile.of(environment.process.variables["DESTACK_PROFILE"]);
    }

    /** The origin of the universe the profile signs people in at by default. */
    get universe(): string {
        return UNIVERSES[this.name];
    }

    /** The suffix of the profile's commands, directories and artwork, empty for a release. */
    get suffix(): string {
        return this.name === "release" ? "" : `-${this.name}`;
    }

    /** The native application identifier, such as `app.destack.desktop.local`. */
    get applicationIdentifier(): string {
        return `app.destack.desktop${this.name === "release" ? "" : `.${this.name}`}`;
    }

    /** The application's display name, such as `Destack Local`. */
    get title(): string {
        return this.name === "release"
            ? "Destack"
            : `Destack ${this.name.charAt(0).toUpperCase()}${this.name.slice(1)}`;
    }

    /** The command the profile installs, such as `destack-local`. */
    get command(): string {
        return `destack${this.suffix}`;
    }

    /** The profile's device directory in a user's home directory, such as `.destack-local`. */
    get directory(): string {
        return `.destack${this.suffix}`;
    }
}

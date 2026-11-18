import type { Environment } from "@destack/environment";
import { UpdateError } from "../error/error.ts";

/** The names of the universes a build belongs to: the published platform, or a local one. */
const NAMES = ["production", "local"] as const;

/** The distribution tag of production's first-party releases, npm's default. */
const PRODUCTION_TAG = "latest";

/** The update feed every published release comes from: the download origin's root. */
const FEED = "https://download.destack.sh/";

/** The origin each universe signs people in at, unless a local one listens elsewhere. */
const ORIGINS = {
    production: "https://destack.app",
    local: "http://127.0.0.1:4100",
} as const satisfies Readonly<Record<(typeof NAMES)[number], string>>;

/** The universe a build belongs to: where people sign in, and its command and identity. */
export class Universe {
    /** The names of the universes a build belongs to. */
    static readonly names = NAMES;

    /** The universe's name. */
    readonly name: (typeof NAMES)[number];
    /** The origin people sign in at. */
    readonly origin: string;

    /** Select a universe by its name, at its own origin unless a local one listens elsewhere. */
    constructor(name: (typeof NAMES)[number], origin: string = ORIGINS[name]) {
        this.name = name;
        this.origin = origin;
    }

    /** Read a universe by its name, refusing any other. */
    static of(name: string | undefined, origin?: string): Universe {
        const found = NAMES.find((candidate) => candidate === name);
        if (found === undefined) {
            throw UpdateError.of(
                "UNIVERSE_INVALID",
                `universe '${name}' is invalid: it is unknown`,
            );
        }

        return new Universe(found, origin);
    }

    /** Read the universe the build selected, and its origin, from the process's variables. */
    static current(environment: Environment<"process">): Universe {
        const { variables } = environment.process;

        return Universe.of(variables["DESTACK_UNIVERSE"], variables["DESTACK_UNIVERSE_ORIGIN"]);
    }

    /** The distribution tag of the first-party releases the universe installs. */
    get tag(): string {
        return this.name === "production" ? PRODUCTION_TAG : this.name;
    }

    /** The update feed the universe's installations follow. */
    get feed(): URL {
        return new URL(FEED);
    }

    /** The suffix of the universe's commands, directories and artwork, empty for production. */
    get suffix(): string {
        return this.name === "production" ? "" : `-${this.name}`;
    }

    /** The native application identifier, such as `app.destack.desktop.local`. */
    get applicationIdentifier(): string {
        return `app.destack.desktop${this.name === "production" ? "" : `.${this.name}`}`;
    }

    /** The application's display name, such as `Destack Local`. */
    get title(): string {
        return this.name === "production"
            ? "Destack"
            : `Destack ${this.name.charAt(0).toUpperCase()}${this.name.slice(1)}`;
    }

    /** The command the universe installs, such as `destack-local`. */
    get command(): string {
        return `destack${this.suffix}`;
    }

    /** The universe's device directory in a user's home directory, such as `.destack-local`. */
    get directory(): string {
        return `.destack${this.suffix}`;
    }
}

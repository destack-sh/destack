import type { Environment } from "@destack/environment";
import { UpdateError } from "../error/error.ts";

/** The names of the stages a build belongs to: the published platform, or the local universe. */
const NAMES = ["production", "local"] as const;

/** The distribution tag of production's first-party releases, npm's default. */
const PRODUCTION_TAG = "latest";

/** The update feed every published release comes from: the download origin's root. */
const FEED = "https://download.destack.sh/";

/** The account service each stage signs people in with. */
const ISSUERS = {
    production: "https://destack.app",
    local: "http://127.0.0.1:4100",
} as const satisfies Readonly<Record<(typeof NAMES)[number], string>>;

/** The stage a build belongs to: its universe, command, directory and application identity. */
export class Stage {
    /** The names of the stages a build belongs to. */
    static readonly names = NAMES;

    /** The stage's name. */
    readonly name: (typeof NAMES)[number];

    /** Select a stage by its name. */
    constructor(name: (typeof NAMES)[number]) {
        this.name = name;
    }

    /** Read a stage by its name, refusing any other. */
    static of(name: string | undefined): Stage {
        const found = NAMES.find((candidate) => candidate === name);
        if (found === undefined) {
            throw UpdateError.of("STAGE_UNKNOWN", `unknown stage ${name}`);
        }

        return new Stage(found);
    }

    /** Read the stage the build selected for this executable, from its environment. */
    static current(environment: Environment<"process">): Stage {
        return Stage.of(environment.process.variables["DESTACK_STAGE"]);
    }

    /** The account service the stage signs people in with. */
    get issuer(): string {
        return ISSUERS[this.name];
    }

    /** The distribution tag of the first-party releases the stage installs. */
    get tag(): string {
        return this.name === "production" ? PRODUCTION_TAG : this.name;
    }

    /** The update feed the stage's installations follow. */
    get feed(): URL {
        return new URL(FEED);
    }

    /** The suffix of the stage's commands, directories and artwork, empty for production. */
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

    /** The command the stage installs, such as `destack-local`. */
    get command(): string {
        return `destack${this.suffix}`;
    }

    /** The stage's device directory name in a user's home directory, such as `.destack-local`. */
    get directory(): string {
        return `.destack${this.suffix}`;
    }
}

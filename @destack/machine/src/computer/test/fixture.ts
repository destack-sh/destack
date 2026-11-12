import { fileURLToPath } from "node:url";

/** The fixture packages worlds install, which the run's setup builds before any world. */
export const WORLD_FIXTURES = ["apps", "board"] as const;

/** The fixture packages only scenarios install, which no template ships. */
export const SCENARIO_FIXTURES = ["mention", "regional"] as const;

/** A fixture package below the machine's tests, which tests build, publish and import. */
export class TestFixture {
    /** The fixture's directory name below the machine's tests, such as `board`. */
    readonly name: string;

    /** Take a fixture by its directory name. */
    constructor(name: string) {
        this.name = name;
    }

    /** The fixture package's directory. */
    get directory(): string {
        return fileURLToPath(new URL(`../../../tests/fixture/${this.name}/`, import.meta.url));
    }
}

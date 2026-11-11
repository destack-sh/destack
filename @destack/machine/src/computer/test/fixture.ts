import { join } from "node:path";
import { fileURLToPath } from "node:url";
import type { FileSystem } from "@destack/environment";
import { schema, Version } from "@destack/schema";

/** The dev build the published fixtures are, as the local stage's first build of a release. */
const DEV_SEQUENCE = 1;

/** The fixture packages worlds install, which the run's setup builds before any world. */
export const WORLD_FIXTURES = ["apps", "board"] as const;

/** The fixture packages only scenarios install, which no template ships. */
export const SCENARIO_FIXTURES = ["mention", "regional"] as const;

/** The name and release a fixture's package.json declares. */
const FixtureManifest = schema.looseObject({
    /** The package's name. */
    name: schema.string().min(1),
    /** The release its dev builds lead to. */
    version: Version,
});

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

    /** Read the package's name and the dev version of the local stage's first build. */
    async version(files: FileSystem): Promise<readonly [string, Version]> {
        const text = await files.readTextFile(join(this.directory, "package.json"));
        const { name, version } = FixtureManifest.parse(JSON.parse(text));

        return [name, Version.parse(`${version}-dev.${DEV_SEQUENCE}`)];
    }

    /** Read the dev versions every published fixture builds and loads at, by package name. */
    static async versions(files: FileSystem): Promise<Record<string, Version>> {
        const names = [...WORLD_FIXTURES, ...SCENARIO_FIXTURES];
        const fixtures = names.map((name) => new TestFixture(name));

        return Object.fromEntries(await Promise.all(fixtures.map((each) => each.version(files))));
    }
}

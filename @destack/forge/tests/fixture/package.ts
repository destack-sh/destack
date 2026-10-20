import accountPackage from "@destack/account/package";
import { onTestFinished } from "vitest";
import { ForgeFixture } from "../../src/test/index.ts";
import { AccountFixture, ids } from "@destack/account/test";
import { cp, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import {
    type BuildOptions,
    PackageBuild,
    Compiler,
    readDependencies,
    readOutputs,
} from "@destack/build";
import { type ImageIndex } from "@destack/db/test";
import { History, Vocabulary } from "@destack/resource";
import { nth, Commit, schema, Version } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { packageObject } from "../../src/object/index.ts";

/** The account that publishes the fixture packages and scopes their npm names. */
export const ACCOUNT_ID = schema
    .identifier("account")
    .parse("account-00000000-0000-7000-8000-000000000001");

/** The version the fixture sources declare. */
export const FIXTURE_VERSION = "2026.9.0";

/** The fixture releases, each dependency before its dependents. */
export const FIXTURE_RELEASES = [
    ["answer", [FIXTURE_VERSION, "2026.9.1"]],
    ["greeting", [FIXTURE_VERSION]],
] as const satisfies readonly (readonly [FixturePackage, readonly string[]])[];

/** The dev prerelease of the answer package after its releases, built without a commit. */
export const PRERELEASE_VERSION = "2026.9.2-dev.1";

/** The commit every fixture release is built from. */
export const COMMIT = "a".repeat(40);

/** The fixture packages: real package sources under tests/fixture called `@example/<name>`. */
export type FixturePackage = "answer" | "greeting";

/** Open a forge publishing as the owner under their account, in a universe of its own. */
export async function openForge(): Promise<ForgeFixture> {
    // serve the universe until the test ends, and install the forge publishing under the owner's account
    const accounts = await openUniverse();
    onTestFinished(() => accounts[Symbol.asyncDispose]());

    return ForgeFixture.create(accounts, { account: ACCOUNT_ID, user: ids.owner });
}

/** Serve the account package with the owner's account, from the run's or given images. */
export async function openUniverse(images?: ImageIndex): Promise<AccountFixture> {
    // serve the account package
    const accounts = await AccountFixture.open({
        dialect: "sqlite",
        policies: [packageObject.policy],
        ...(images === undefined ? {} : { images }),
    });

    // create the owner's account
    await accounts.user(ids.owner, accountPackage).account.create({
        scope: ids.owner,
        id: ACCOUNT_ID,
        requestId: RequestId.create(Date.now()),
        handle: "example",
        name: "Example",
        kind: "shared",
    });

    return accounts;
}

/** A copy of a fixture package's source with a started builder. */
export class FixtureSource implements AsyncDisposable {
    /** The fixture package. */
    readonly name: FixturePackage;
    /** The directory with the replica. */
    readonly directory: string;
    /** The builder retained across the package's versions. */
    readonly #builder: Compiler;
    /** The outputs and dependencies the last build compiled, and what it published. */
    #compiled: (Pick<BuildOptions, "outputs" | "dependencies"> & { history: History }) | undefined;

    /** Take a replica and its started builder. */
    private constructor(name: FixturePackage, directory: string, builder: Compiler) {
        this.name = name;
        this.directory = directory;
        this.#builder = builder;
    }

    /** Replicate a fixture package's source into a directory and start its builder. */
    static async open(name: FixturePackage, parent: string): Promise<FixtureSource> {
        const directory = join(parent, name);
        await cp(fileURLToPath(new URL(`./${name}/`, import.meta.url)), directory, {
            recursive: true,
        });

        return new FixtureSource(name, directory, await Compiler.start(directory));
    }

    /** Install the dependencies and build each version's published outputs. */
    async build(npmrc: string, versions: readonly string[]): Promise<PackageBuild[]> {
        // install the dependencies
        await writeFile(join(this.directory, ".npmrc"), npmrc);
        await run(
            [
                process.execPath,
                "install",
                "--cache-dir",
                join(this.directory, "..", "cache", this.name),
            ],
            this.directory,
        );
        const declaration = schema
            .jsonObject()
            .parse(await readJson(join(this.directory, "package.json")));
        const dependencies = await readDependencies(this.directory);

        // build one unbundled module per runtime the package ships, as a build worker does
        const outputs = Object.fromEntries(
            Object.entries(await readOutputs(this.directory)).map(([name, { kind, runtime }]) => [
                name,
                { kind, runtime },
            ]),
        );

        // build each version with the retained builder, planned from the version before
        const builds: PackageBuild[] = [];
        let vocabulary: Vocabulary = {};
        for (const version of versions) {
            await writeFile(
                join(this.directory, "package.json"),
                JSON.stringify({ ...declaration, version }),
            );
            const latest = builds.at(-1);
            const history =
                latest === undefined ? undefined : await History.read(latest.reader, vocabulary);
            const build = await this.#builder.build({
                outputs,
                dependencies,
                commit: Commit.parse(COMMIT),
                ...(history === undefined ? {} : { history }),
            });
            vocabulary = Vocabulary.advance(vocabulary, await build.reader.declarations(), version);
            builds.push(build);
        }

        const latest = nth(builds, -1);
        this.#compiled = {
            outputs,
            dependencies,
            history: await History.read(latest.reader, vocabulary),
        };

        return builds;
    }

    /** Build a working tree without a commit: the last version, or a prerelease of the next. */
    async uncommitted(prerelease?: Version): Promise<PackageBuild> {
        if (this.#compiled === undefined) {
            throw new TypeError(`fixture ${this.name} has not been built`);
        }

        // build the last version again
        const { outputs, dependencies, history } = this.#compiled;
        if (prerelease === undefined) {
            return await this.#builder.build({ outputs, dependencies });
        }

        // author the next version, and build its prerelease planned from the last version
        const path = join(this.directory, "package.json");
        const declaration = schema.jsonObject().parse(await readJson(path));
        const version = Version.next(Version.parse(declaration["version"]));
        await writeFile(path, JSON.stringify({ ...declaration, version }));
        const name = schema.string().parse(declaration["name"]);

        return await this.#builder.build({
            outputs,
            dependencies,
            history,
            versions: { [name]: prerelease },
        });
    }

    /** Stop the builder. */
    [Symbol.asyncDispose](): Promise<void> {
        return this.#builder[Symbol.asyncDispose]();
    }
}

/** Run a command in a directory, failing with its error output unless it succeeds. */
export async function run(command: readonly string[], directory: string): Promise<string> {
    // run the command, collecting both outputs
    const child = Bun.spawn([...command], {
        cwd: directory,
        stdout: "pipe",
        stderr: "pipe",
        timeout: 60_000,
    });
    const [code, stdout, stderr] = await Promise.all([
        child.exited,
        new Response(child.stdout).text(),
        new Response(child.stderr).text(),
    ]);
    if (code !== 0) {
        throw new Error(`${command.join(" ")} failed with ${code}: ${stderr}`);
    }

    return stdout;
}

/** Read a JSON file. */
export async function readJson(path: string): Promise<unknown> {
    return JSON.parse(await readFile(path, "utf8"));
}

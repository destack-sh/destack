import type { SimulationOptions } from "@destack/environment/simulation";
import { Digest, type JsonValue, present, schema } from "@destack/schema";
import { Failure } from "@destack/error";
import { dirname, join, relative } from "node:path";
import { testHost } from "@destack/test/host";
import { fileURLToPath } from "node:url";
import postgres from "postgres";
import { inject, type TestProject } from "@destack/test";
import { Dialect } from "../dialect/dialect.ts";
import { TestDatabase } from "./database.ts";
import { ImageRecord, type TestTemplate, type TemplateDatabases } from "./template.ts";

/** The host the run keeps its images on, shared by the test processes of a run. */
const HOST = testHost();

/** The directory of built images below the person's cache directory. */
const CACHE = join("destack", "templates");

/** The hex digits of a checkout root's digest naming its templates' directory: 64 bits. */
const CHECKOUT_DIGITS = 16;

/** The file every workspace root has, which the checkout's root is found by. */
const LOCKFILE = "bun.lock";

/** The hexadecimal characters of a key in its schemas' names: 64 bits. */
const ID_LENGTH = 16;

/** The default longest build of one template: 60 s, about 8x the longest measured, 7.1 s. */
const BUILD_MILLISECONDS = 60_000;

/** The hex digits of a template name's digest in its image's file name: 64 bits. */
const NAME_DIGITS = 16;

/** Milliseconds per second. */
const MILLISECONDS_PER_SECOND = 1000;

/** The files the key reads at once: well below the default limit of 256 open files on macOS. */
const READ_BATCH = 64;

/** The fields of a workspace's root manifest the key reads. */
const WorkspaceManifest = schema.looseObject({ workspaces: schema.array(schema.string()) });

/** The fields of a package's manifest the key reads. */
const PackageManifest = schema.looseObject({
    name: schema.string(),
    dependencies: schema.record(schema.string(), schema.string()).exactOptional(),
    devDependencies: schema.record(schema.string(), schema.string()).exactOptional(),
    peerDependencies: schema.record(schema.string(), schema.string()).exactOptional(),
});

/** Where a run keeps its images and what they are of, as its setup provides it to the tests. */
export const RunRecord = schema.object({
    /** The directory of the run's images, one per key of its inputs. */
    directory: schema.string(),
    /** The key's first digits, which start the names of the images' PostgreSQL schemas. */
    id: schema.string(),
    /** The database the images are of. */
    dialect: Dialect,
    /** The longest one template's build may take, in milliseconds. */
    deadline: schema.number(),
    /** When the run opened, in UTC epoch milliseconds, which marks the builds that failed in it. */
    openedAt: schema.number(),
    /** The variables the images depend on beside the sources, which template builds read. */
    variables: schema.record(schema.string(), schema.string()),
});
/** Where a run keeps its images and what they are of. */
export type RunRecord = schema.Infer<typeof RunRecord>;

/** A template's build that failed in a run, which the run's other tests fail with at once. */
const BuildFailure = schema.object({
    /** When the run it failed in opened. */
    openedAt: schema.number(),
    /** Why it failed, with its causes. */
    message: schema.string(),
});

/** The run's provided values the templates read. */
declare module "vitest" {
    /** The values a run's setup provides to its tests. */
    export interface ProvidedContext {
        /** Where the run keeps its images, absent without a setup. */
        run?: RunRecord;
    }
}

/** A template's built image in a run, which tests fork. */
export class TestImage<
    Value extends JsonValue = JsonValue,
    Databases extends TemplateDatabases = TemplateDatabases,
> {
    /** The template the image is of. */
    readonly template: TestTemplate<Value, Databases>;
    /** What the build left: where the rows and files are, its value and its end. */
    readonly record: ImageRecord;

    /** Take a template's image as its build left it. */
    constructor(template: TestTemplate<Value, Databases>, record: ImageRecord) {
        this.template = template;
        this.record = record;
    }

    /** Fork the image for a test: its world at the build's end, its databases copied, its value. */
    fork(
        label: string,
        options: Pick<SimulationOptions, "isBridged"> = {},
    ): ReturnType<typeof TestDatabase.fork<Value, Databases>> {
        return TestDatabase.fork(this.template, this.record, label, options);
    }
}

/** The images this process read or built, by run directory and template name. */
const IMAGES = new Map<string, Promise<ImageRecord>>();

/** A test run: its images, built when first forked and reused while their inputs stay the same. */
export const TestRun = {
    /** Open a run for a package's tests, keyed by its sources and some variables. */
    setup(module: URL, variables: Readonly<Record<string, string>> = {}) {
        return async (project: TestProject): Promise<() => Promise<void>> => {
            const run = await TestRun.open(module, TestDatabase.target(project), { variables });
            project.provide("run", run.record);

            return run.release;
        };
    },

    /** Open the run of a module's package for a database, used until released. */
    async open(
        module: URL,
        dialect: Dialect,
        options: RunOptions = {},
    ): Promise<{ readonly record: RunRecord; readonly release: () => Promise<void> }> {
        // open the cache keyed by the package's sources and the variables, and hold it in use
        const { variables = {}, deadline = BUILD_MILLISECONDS } = options;
        const cache = await ImageCache.open(fileURLToPath(module), dialect, variables);
        const release = await cache.use();

        return {
            record: {
                directory: cache.directory,
                id: cache.id,
                dialect,
                deadline,
                openedAt: HOST.clock.now(),
                variables,
            },
            release,
        };
    },

    /** Read a template's image in a run, building it and its parents the first time. */
    async image<Value extends JsonValue, Databases extends TemplateDatabases>(
        template: TestTemplate<Value, Databases>,
        given?: RunRecord,
    ): Promise<TestImage<Value, Databases>> {
        // require a run, and an image of its database
        const run = given ?? inject("run");
        if (run === undefined) {
            throw new TypeError(
                `no run to read the image of template '${template.name}' in: set up TestRun in its package's test setup`,
            );
        } else if (template.dialects?.includes(run.dialect) === false) {
            throw new TypeError(`template '${template.name}' has no '${run.dialect}' image`);
        }

        // read or build it once per process
        const key = `${run.directory} ${template.name}`;
        const known = IMAGES.get(key) ?? ImageCache.image(template, run);
        IMAGES.set(key, known);
        known.catch(() => IMAGES.delete(key));

        return new TestImage(template, await known);
    },

    /** Drop the PostgreSQL schemas of a run's images. */
    async drop(id: string, dialect: Dialect): Promise<void> {
        // leave a SQLite run's images to its directory
        if (dialect === "sqlite") {
            return;
        }

        // drop each schema of the run
        const session = postgres(TestDatabase.address(), { max: 1, fetch_types: false });
        const schemas = await session<{ name: string }[]>`
            SELECT nspname AS name FROM pg_namespace WHERE starts_with(nspname, ${`test_t${id}_`})`;
        for (const { name } of schemas) {
            await session.unsafe(`DROP SCHEMA "${name}" CASCADE`);
        }
        await session.end();
    },
};

/** How a run is keyed and how long its builds may take. */
export interface RunOptions {
    /** The variables the images depend on beside the sources, none by default. */
    readonly variables?: Readonly<Record<string, string>>;
    /** The longest one template's build may take in milliseconds, 60 s by default. */
    readonly deadline?: number;
}

/** The images of one package's run for one database, in a directory keyed by their inputs. */
class ImageCache {
    /** The database the images are for. */
    readonly dialect: Dialect;
    /** The directory of the package's entries, one per key. */
    readonly entries: string;
    /** The digest of every input of the images. */
    readonly key: Digest;

    /** Describe a package's cache in its entries' directory, keyed by its inputs. */
    private constructor(dialect: Dialect, entries: string, key: Digest) {
        // keep its database, its key and its entries' directory
        this.dialect = dialect;
        this.key = key;
        this.entries = entries;
    }

    /** Open the cache of a module's package, keyed by the digest of its inputs. */
    static async open(
        module: string,
        dialect: Dialect,
        variables: Readonly<Record<string, string>>,
    ): Promise<ImageCache> {
        // find the checkout's root, the module's package and the packages it depends on
        const root = await ImageCache.#root(module);
        const packages = await ImageCache.#dependencies(module, root);

        // key the inputs: the database, the runtime, the checkout, the module and the sources
        const digests = await ImageCache.#digests(root, packages);
        const key = await Digest.json(
            {
                dialect,
                server: dialect === "postgresql" ? TestDatabase.address() : null,
                runtime: HOST.process.runtime.revision,
                root,
                module: relative(root, module),
                digests,
                variables,
            },
            HOST.crypto,
        );

        // place the package's entries in the person's cache, apart per checkout and database
        const checkout = (await Digest.json(root, HOST.crypto)).slice(0, CHECKOUT_DIGITS);
        const name = relative(root, module).replaceAll("/", "_");
        const cache = join(await HOST.files.cacheDirectory(), CACHE);
        const entries = join(cache, checkout, name, dialect);

        return new ImageCache(dialect, entries, key);
    }

    /** The directory of the images for the current key. */
    get directory(): string {
        return join(this.entries, this.key);
    }

    /** The schema name prefix of the current key's images. */
    get id(): string {
        return this.key.slice(0, ID_LENGTH);
    }

    /** Use the current key's entry until released, removing other keys' entries no run uses. */
    async use(): Promise<() => Promise<void>> {
        // hold the entry shared while in use, which every cleanup then leaves
        await HOST.files.mkdir(this.directory, { recursive: true });
        const using = await HOST.files.lock(`${this.directory}.use`, "shared");
        try {
            // drop the schemas of ended runs, then remove other keys' entries
            if (this.dialect === "postgresql") {
                await TestDatabase.sweep();
                await TestDatabase.close();
            }
            await this.#clean();

            return () => using.close();
        } catch (error) {
            await using.close();
            throw error;
        }
    }

    /** Read a template's image in a run, building it under its lock when it is missing. */
    static async image<Value extends JsonValue, Databases extends TemplateDatabases>(
        template: TestTemplate<Value, Databases>,
        run: RunRecord,
    ): Promise<ImageRecord> {
        // read a built image, one builder at a time otherwise
        const name = (await Digest.json(template.name, HOST.crypto)).slice(0, NAME_DIGITS);
        const file = join(run.directory, `image-${name}.json`);
        const built = await ImageCache.#read(file);
        if (built !== undefined) {
            return built;
        }
        await using _building = await HOST.files.lock(`${file}.build`, "exclusive");
        const rebuilt = await ImageCache.#read(file);
        if (rebuilt !== undefined) {
            return rebuilt;
        }

        // fail at once with a build's failure earlier in this run, as its inputs are the same
        const failure = `${file}.failed`;
        if (await HOST.files.exists(failure)) {
            const failed = BuildFailure.parse(JSON.parse(await HOST.files.readTextFile(failure)));
            if (failed.openedAt === run.openedAt) {
                throw new Error(`building template '${template.name}' failed: ${failed.message}`);
            }
        }

        // build it, recording a failure for the run's other tests
        try {
            return await ImageCache.#build(template, run, file);
        } catch (error) {
            // record why with the failure's causes
            const messages = Failure.causes(error).flatMap((each) => {
                // keep each cause's message, with the data a refusal carries
                const data: unknown = "data" in each ? each.data : undefined;
                const described =
                    data === undefined ? each.message : `${each.message} ${JSON.stringify(data)}`;

                return described === "" ? [] : [described];
            });
            const failed = { openedAt: run.openedAt, message: messages.join(": ") };
            await HOST.files.writeTextFile(failure, JSON.stringify(failed));
            throw error;
        }
    }

    /** Read a built image's record and claim its schemas, absent when unbuilt or not whole. */
    static async #read(file: string): Promise<ImageRecord | undefined> {
        // read the record
        if (!(await HOST.files.exists(file))) {
            return undefined;
        }
        const record = ImageRecord.parse(JSON.parse(await HOST.files.readTextFile(file)));

        // claim its PostgreSQL schemas, which every sweep then skips
        const sources = [...Object.values(record.sources), ...Object.values(record.volumes)];
        const schemas = sources.flatMap((source) =>
            source.dialect === "postgresql" ? [source.schema] : [],
        );
        const isWhole = schemas.length === 0 || (await TestDatabase.claimImage(schemas));

        return isWhole ? record : undefined;
    }

    /** Build a template's image on its parent's, within the run's deadline, and record it last. */
    static async #build<Value extends JsonValue, Databases extends TemplateDatabases>(
        template: TestTemplate<Value, Databases>,
        run: RunRecord,
        file: string,
    ): Promise<ImageRecord> {
        // read or build the parent's image first
        const started = HOST.clock.now();
        const parent =
            template.from === undefined ? undefined : await ImageCache.image(template.from, run);

        // build until the deadline stops it
        const deadline = new AbortController();
        const seconds = run.deadline / MILLISECONDS_PER_SECOND;
        const built = new AbortController();
        void HOST.clock.sleep(run.deadline, built.signal).then(
            () =>
                deadline.abort(
                    new Error(`building template '${template.name}' took over ${seconds} s`),
                ),
            () => {},
        );
        let record: ImageRecord;
        try {
            record = await TestDatabase.build(template, run.dialect, run, parent, deadline.signal);
        } finally {
            built.abort();
        }

        // record it once whole, then report its time
        await HOST.files.writeTextFile(`${file}.building`, JSON.stringify(record));
        await HOST.files.rename(`${file}.building`, file);
        const taken = ((HOST.clock.now() - started) / MILLISECONDS_PER_SECOND).toFixed(1);
        HOST.process.stderr.writeSync(`template ${template.name}: ${taken} s\n`);

        return present(await ImageCache.#read(file), `the built image of '${template.name}'`);
    }

    /** Remove the package's entries of other keys that no run uses. */
    async #clean(): Promise<void> {
        for (const entry of await HOST.files.readDirectory(this.entries)) {
            // lock each other entry exclusive now, else leave it to the runs using it
            const isOther = entry.kind === "directory" && entry.name !== this.key;
            const path = join(this.entries, entry.name);
            const used = isOther ? await HOST.files.tryLock(`${path}.use`, "exclusive") : undefined;

            // remove it, leaving its lock file, which a waiting process may have open
            if (used !== undefined) {
                await this.#remove(path, entry.name.slice(0, ID_LENGTH));
            }
            await used?.close();
        }
    }

    /** Remove an entry's directory and PostgreSQL schemas. */
    async #remove(directory: string, id: string): Promise<void> {
        await TestRun.drop(id, this.dialect);
        await HOST.files.remove(directory, { recursive: true, force: true });
    }

    /** Find the checkout's root above a path, the directory with the workspace's lockfile. */
    static async #root(path: string): Promise<string> {
        let directory = dirname(path);
        while (!(await HOST.files.exists(join(directory, LOCKFILE)))) {
            const parent = dirname(directory);
            if (parent === directory) {
                throw new Error(`'${path}' is in no workspace with a '${LOCKFILE}'`);
            }
            directory = parent;
        }

        return directory;
    }

    /** List the directories of a path's package and its workspace dependencies, transitively. */
    static async #dependencies(path: string, root: string): Promise<string[]> {
        // index the workspace's packages by name, across every pattern
        const workspace = WorkspaceManifest.parse(await readJson(join(root, "package.json")));
        const named = new Map<string, string>();
        for (const pattern of workspace.workspaces) {
            for (const directory of await ImageCache.#expand(root, pattern.split("/"))) {
                const manifest = join(directory, "package.json");
                if (await HOST.files.exists(manifest)) {
                    const { name } = PackageManifest.parse(await readJson(manifest));
                    named.set(name, directory);
                }
            }
        }

        // walk from the path's package through every workspace dependency once
        const start = await ImageCache.#package(path, root);
        const visited = new Set<string>([start]);
        const pending = [start];
        for (let next = pending.pop(); next !== undefined; next = pending.pop()) {
            const manifest = PackageManifest.parse(await readJson(join(next, "package.json")));
            const dependencies = Object.entries({
                ...manifest.peerDependencies,
                ...manifest.devDependencies,
                ...manifest.dependencies,
            });
            for (const [name, range] of dependencies) {
                const found = range.startsWith("workspace:") ? named.get(name) : undefined;
                if (found !== undefined && !visited.has(found)) {
                    visited.add(found);
                    pending.push(found);
                }
            }
        }

        return [...visited].map((each) => relative(root, each)).toSorted();
    }

    /** List the directories a workspace pattern's segments match below a directory, `*` any one. */
    static async #expand(directory: string, segments: readonly string[]): Promise<string[]> {
        // match the directory itself once no segment is left
        const [segment, ...rest] = segments;
        if (segment === undefined) {
            return [directory];
        }
        // descend into a named directory
        else if (segment !== "*") {
            const next = join(directory, segment);

            return (await HOST.files.exists(next)) ? ImageCache.#expand(next, rest) : [];
        }
        // descend into every directory
        else {
            const entries = await HOST.files.readDirectory(directory);
            const matched = await Promise.all(
                entries
                    .filter((entry) => entry.kind === "directory")
                    .map((entry) => ImageCache.#expand(join(directory, entry.name), rest)),
            );

            return matched.flat();
        }
    }

    /** Find the directory of the package a path belongs to, within the checkout. */
    static async #package(path: string, root: string): Promise<string> {
        let directory = dirname(path);
        while (!(await HOST.files.exists(join(directory, "package.json")))) {
            if (directory === root) {
                throw new Error(`'${path}' belongs to no package below '${root}'`);
            }
            directory = dirname(directory);
        }

        return directory;
    }

    /** Hash every non-test source of some packages, and the lockfile, by path. */
    static async #digests(
        root: string,
        packages: readonly string[],
    ): Promise<(readonly [string, Digest | null])[]> {
        // list the tracked and untracked files git does not ignore
        const command = ["git", "-C", root, "ls-files", "-z", "--cached", "--others"];
        const listing = HOST.process.spawn([...command, "--exclude-standard", "--", ...packages], {
            stdout: "pipe",
            stderr: "pipe",
        });
        const output = await listing.output();
        if (output.code !== 0) {
            const stderr = new TextDecoder().decode(output.stderr);
            throw new Error(`listing the template sources failed: '${stderr}'`);
        }
        const files = new TextDecoder()
            .decode(output.stdout)
            .split("\0")
            .filter((file) => file !== "" && !file.endsWith(".test.ts") && !file.endsWith(".md"))
            .toSorted();

        // hash each file in batches, a listed file deleted since as absent
        const digests: (readonly [string, Digest | null])[] = [];
        const listed = [...files, LOCKFILE];
        for (let start = 0; start < listed.length; start += READ_BATCH) {
            const batch = listed.slice(start, start + READ_BATCH);
            const hashed = await Promise.all(
                batch.map(async (file) => {
                    const path = join(root, file);
                    const digest = (await HOST.files.exists(path))
                        ? await Digest.of(await HOST.files.readFile(path), HOST.crypto)
                        : null;

                    return [file, digest] as const;
                }),
            );
            digests.push(...hashed);
        }

        return digests;
    }
}

/** Read a JSON file. */
async function readJson(path: string): Promise<unknown> {
    const parsed: unknown = JSON.parse(await HOST.files.readTextFile(path));

    return parsed;
}

import { Digest, schema } from "@destack/schema";
import { dirname, join, relative } from "node:path";
import { testHost } from "@destack/test/host";
import { fileURLToPath, pathToFileURL } from "node:url";
import postgres from "postgres";
import type { TestProject } from "@destack/test";
import type { Dialect } from "../dialect/dialect.ts";
import { TestDatabase } from "./database.ts";
import { ImageIndex } from "./template.ts";

export type { ImageIndex } from "./template.ts";

/** The host the run's setup writes templates on, as the run's entrypoint. */
const HOST = testHost();

/** The file the writer leaves the index of the run's template images in. */
const INDEX = "images.json";

/** The directory of written templates below a checkout's root, as find-cache-dir places it. */
const CACHE = join("node_modules", ".cache", "destack", "templates");

/** The file every workspace root has, which the checkout's root is found by. */
const LOCKFILE = "bun.lock";

/** The hexadecimal characters of a key in its schemas' names: 64 bits. */
const ID_LENGTH = 16;

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

/** The templates a test run copies, reused by every run while their inputs stay the same. */
export const TestRun = {
    /** Provide a module's templates to a project's tests, released at the run's end. */
    setup(module: URL) {
        return async (project: TestProject): Promise<() => Promise<void>> => {
            const run = await TestRun.write(module, TestDatabase.target(project));
            project.provide("templates", run.images);

            return run.release;
        };
    },

    /** Use a module's templates for a database, writing them first when their inputs changed. */
    async write(
        module: URL,
        dialect: Dialect,
    ): Promise<{
        readonly images: ImageIndex;
        readonly release: () => Promise<void>;
    }> {
        const cache = await TemplateCache.open(fileURLToPath(module), dialect);

        return await cache.use();
    },

    /** Read the index of a run's template images from its file. */
    async read(index: string): Promise<ImageIndex> {
        const parsed: unknown = JSON.parse(await HOST.files.readTextFile(index));

        return ImageIndex.parse(parsed);
    },

    /** Drop the PostgreSQL schemas of a run's template images. */
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

/** The written images of one template module for one database, keyed by their inputs. */
class TemplateCache {
    /** The template module's path. */
    readonly module: string;
    /** The database the images are for. */
    readonly dialect: Dialect;
    /** The directory of the module's entries, one per key. */
    readonly entries: string;
    /** The digest of every input of the images. */
    readonly key: Digest;

    /** Describe a module's cache below its checkout's root, keyed by its inputs. */
    private constructor(module: string, dialect: Dialect, root: string, key: Digest) {
        // place the module's entries below the root, one directory per database
        this.module = module;
        this.dialect = dialect;
        this.key = key;
        this.entries = join(root, CACHE, relative(root, module).replaceAll("/", "_"), dialect);
    }

    /** Open a module's cache, reading its checkout's root and the key of its inputs. */
    static async open(module: string, dialect: Dialect): Promise<TemplateCache> {
        // find the checkout's root, the module's package and the packages it depends on
        const root = await TemplateCache.#root(module);
        const packages = await TemplateCache.#closure(module, root);

        // key the inputs: the database, the runtime, the checkout, the module and the sources
        const digests = await TemplateCache.#digests(root, packages);
        const key = await Digest.json(
            {
                dialect,
                server: dialect === "postgresql" ? TestDatabase.address() : null,
                runtime: HOST.process.runtime.revision,
                root,
                module: relative(root, module),
                digests,
            },
            HOST.crypto,
        );

        return new TemplateCache(module, dialect, root, key);
    }

    /** The directory of the images for the current key. */
    get directory(): string {
        return join(this.entries, this.key);
    }

    /** The schema name prefix of the current key's images. */
    get id(): string {
        return this.key.slice(0, ID_LENGTH);
    }

    /** Use the current key's images, writing them first when missing or incomplete. */
    async use(): Promise<{ readonly images: ImageIndex; readonly release: () => Promise<void> }> {
        // hold the entry shared while in use, which every cleanup then leaves
        await HOST.files.mkdir(this.entries, { recursive: true });
        const using = await HOST.files.lock(`${this.directory}.use`, "shared");
        try {
            // write the images unless complete, one writer at a time
            let images = await this.#complete();
            if (images === undefined) {
                await using _writing = await HOST.files.lock(
                    `${this.directory}.write`,
                    "exclusive",
                );
                images = (await this.#complete()) ?? (await this.#write());
            }

            // claim the images' schemas shared until the run ends, then remove other keys' entries
            const claims = await TemplateCache.#claim(images, this.dialect);
            await this.#clean();

            return {
                images,
                release: async () => {
                    await claims?.end();
                    await using.close();
                },
            };
        } catch (error) {
            await using.close();
            throw error;
        }
    }

    /** Read the current key's images when written in full with every schema present. */
    async #complete(): Promise<ImageIndex | undefined> {
        // read a written index
        const index = join(this.directory, INDEX);
        if (!(await HOST.files.exists(index))) {
            return undefined;
        }
        const images = await TestRun.read(index);

        return (await TemplateCache.#restamp(images, this.dialect)) ? images : undefined;
    }

    /** Write the current key's images in a child, after removing an earlier failed write. */
    async #write(): Promise<ImageIndex> {
        // clear an earlier write, then write in a child
        await this.#remove(this.directory, this.id);
        await HOST.files.mkdir(this.directory, { recursive: true });
        const writer = fileURLToPath(new URL("./write.ts", import.meta.url));
        const module = pathToFileURL(this.module).href;
        const arguments_ = [writer, module, this.directory, this.id, this.dialect];
        const child = HOST.process.spawn([HOST.process.executable, ...arguments_], {
            stdin: "pipe",
            stdout: "inherit",
            stderr: "pipe",
        });
        const [code, stderr] = await Promise.all([child.exited, new Response(child.stderr).text()]);

        // remove a failed write's images
        if (code !== 0) {
            await this.#remove(this.directory, this.id);
            throw new Error(`writing the run's templates failed with ${code}: ${stderr}`);
        }

        return await TestRun.read(join(this.directory, INDEX));
    }

    /** Remove the module's entries of other keys that no run uses or writes. */
    async #clean(): Promise<void> {
        for (const entry of await HOST.files.readDirectory(this.entries)) {
            // lock each other entry exclusive now, else leave it to its users
            const isOther = entry.kind === "directory" && entry.name !== this.key;
            const path = join(this.entries, entry.name);
            const used = isOther ? await HOST.files.tryLock(`${path}.use`, "exclusive") : undefined;
            const written =
                used === undefined
                    ? undefined
                    : await HOST.files.tryLock(`${path}.write`, "exclusive");

            // remove it, leaving its lock files, which a waiting process may have open
            if (used !== undefined && written !== undefined) {
                await this.#remove(path, entry.name.slice(0, ID_LENGTH));
            }
            await written?.close();
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
                throw new Error(`${path} is in no workspace with a ${LOCKFILE}`);
            }
            directory = parent;
        }

        return directory;
    }

    /** List the directories of a path's package and its workspace dependencies, transitively. */
    static async #closure(path: string, root: string): Promise<string[]> {
        // index the workspace's packages by name, across every pattern
        const workspace = WorkspaceManifest.parse(await readJson(join(root, "package.json")));
        const named = new Map<string, string>();
        for (const pattern of workspace.workspaces) {
            for (const directory of await TemplateCache.#expand(root, pattern.split("/"))) {
                const manifest = join(directory, "package.json");
                if (await HOST.files.exists(manifest)) {
                    const { name } = PackageManifest.parse(await readJson(manifest));
                    named.set(name, directory);
                }
            }
        }

        // walk from the path's package through every workspace dependency once
        const start = await TemplateCache.#package(path, root);
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

            return (await HOST.files.exists(next)) ? TemplateCache.#expand(next, rest) : [];
        }
        // descend into every directory
        else {
            const entries = await HOST.files.readDirectory(directory);
            const matched = await Promise.all(
                entries
                    .filter((entry) => entry.kind === "directory")
                    .map((entry) => TemplateCache.#expand(join(directory, entry.name), rest)),
            );

            return matched.flat();
        }
    }

    /** Find the directory of the package a path belongs to, within the checkout. */
    static async #package(path: string, root: string): Promise<string> {
        let directory = dirname(path);
        while (!(await HOST.files.exists(join(directory, "package.json")))) {
            if (directory === root) {
                throw new Error(`${path} belongs to no package below ${root}`);
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
            throw new Error(`listing the template sources failed: ${stderr}`);
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

    /** Restamp the PostgreSQL schemas of written images, answering whether every one exists. */
    static async #restamp(images: ImageIndex, dialect: Dialect): Promise<boolean> {
        // take SQLite images as their directory has them
        const schemas = TemplateCache.#schemas(images);
        if (dialect === "sqlite" || schemas.length === 0) {
            return true;
        }

        // restamp each schema, which spares it other runs' sweeps for another retention
        const session = postgres(TestDatabase.address(), { max: 1, fetch_types: false });
        try {
            const found = await session<{ name: string }[]>`
                SELECT nspname AS name FROM pg_namespace WHERE nspname IN ${session(schemas)}`;
            if (found.length !== schemas.length) {
                return false;
            }
            for (const name of schemas) {
                await session.unsafe(`COMMENT ON SCHEMA "${name}" IS '${HOST.clock.now()} image'`);
            }

            return true;
        } finally {
            await session.end();
        }
    }

    /** Claim the PostgreSQL schemas of images shared, which every sweep's claim then skips. */
    static async #claim(images: ImageIndex, dialect: Dialect): Promise<postgres.Sql | undefined> {
        // claim nothing for SQLite images
        const schemas = TemplateCache.#schemas(images);
        if (dialect === "sqlite" || schemas.length === 0) {
            return undefined;
        }

        // hold a shared advisory lock on each schema until the session ends
        const session = postgres(TestDatabase.address(), {
            max: 1,
            max_lifetime: null,
            fetch_types: false,
        });
        for (const name of schemas) {
            await session`SELECT pg_advisory_lock_shared(hashtext(${name}))`.values();
        }

        return session;
    }

    /** List the distinct PostgreSQL schemas of images. */
    static #schemas(images: ImageIndex): string[] {
        const schemas = Object.values(images).flatMap((image) =>
            Object.values(image.postgresql?.sources ?? {}).flatMap((source) =>
                source.dialect === "postgresql" ? [source.schema] : [],
            ),
        );

        return [...new Set(schemas)];
    }
}

/** Read a JSON file. */
async function readJson(path: string): Promise<unknown> {
    const parsed: unknown = JSON.parse(await HOST.files.readTextFile(path));

    return parsed;
}

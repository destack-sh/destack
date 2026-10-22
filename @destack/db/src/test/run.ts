import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import postgres from "postgres";
import type { TestProject } from "@destack/test";
import type { Dialect } from "../dialect/dialect.ts";
import { TestDatabase } from "./database.ts";
import { ImageIndex } from "./template.ts";

export type { ImageIndex } from "./template.ts";

/** The file the writer leaves the index of the run's template images in. */
const INDEX = "images.json";

/** The templates a test run writes once before its workers start, and removes at its end. */
export const TestRun = {
    /** Write a module's templates once per run and provide their images, as a run's setup. */
    setup(module: URL) {
        return async (project: TestProject): Promise<() => Promise<void>> => {
            const run = await TestRun.write(module, TestDatabase.target(project));
            project.provide("templates", run.images);

            return run.remove;
        };
    },

    /** Write a module's templates for a database in a child process. */
    async write(
        module: URL,
        dialect: Dialect,
    ): Promise<{
        readonly images: ImageIndex;
        readonly index: string;
        readonly remove: () => Promise<void>;
    }> {
        // write the templates' images in a child, into the run's directory
        const directory = await mkdtemp(join(tmpdir(), "destack-run-"));
        const id = crypto.randomUUID().replaceAll("-", "").slice(0, 16);
        const writer = fileURLToPath(new URL("./write.ts", import.meta.url));
        const child = Bun.spawn([process.execPath, writer, module.href, directory, id, dialect], {
            stdout: "inherit",
            stderr: "pipe",
        });
        const [code, stderr] = await Promise.all([child.exited, new Response(child.stderr).text()]);

        // remove the run's images once it ends, or at once after a failed write
        const remove = async () => {
            await TestRun.drop(id, dialect);
            await rm(directory, { recursive: true, force: true });
        };
        if (code !== 0) {
            await remove();
            throw new Error(`writing the run's templates failed with ${code}: ${stderr}`);
        }

        // read the images' index the writer left
        const index = join(directory, INDEX);

        return { images: await TestRun.read(index), index, remove };
    },

    /** Read the index of a run's template images from its file. */
    async read(index: string): Promise<ImageIndex> {
        const parsed: unknown = JSON.parse(await readFile(index, "utf8"));

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

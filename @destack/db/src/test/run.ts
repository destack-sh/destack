import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import postgres from "postgres";
import { ImageIndex } from "./template.ts";

/** The file the writer leaves the index of the run's template images in. */
const INDEX = "images.json";

/** The templates a test run writes once before its workers start, and removes at its end. */
export const TestRun = {
    /** Write the templates a module lists once per run and provide them, as a run's setup does. */
    setup(module: URL) {
        return async (project: {
            readonly provide: (key: "templates", value: ImageIndex) => void;
        }): Promise<() => Promise<void>> => {
            // write the templates in a child with the runtime's module resolution, into the run's directory
            const directory = await mkdtemp(join(tmpdir(), "destack-run-"));
            const id = crypto.randomUUID().replaceAll("-", "").slice(0, 16);
            const writer = fileURLToPath(new URL("./write.ts", import.meta.url));
            const child = Bun.spawn([process.execPath, writer, module.href, directory, id], {
                stdout: "inherit",
                stderr: "pipe",
            });
            const [code, stderr] = await Promise.all([
                child.exited,
                new Response(child.stderr).text(),
            ]);

            // remove the run's templates once the run ends, or at once after a failed write
            const remove = async () => {
                await TestRun.drop(id);
                await rm(directory, { recursive: true, force: true });
            };
            if (code !== 0) {
                await remove();
                throw new Error(`writing the run's templates failed with ${code}: ${stderr}`);
            }

            // provide the images to the tests
            const index: unknown = JSON.parse(await readFile(join(directory, INDEX), "utf8"));
            project.provide("templates", ImageIndex.parse(index));

            return remove;
        };
    },

    /** Drop the PostgreSQL schemas of a run's template images. */
    async drop(id: string): Promise<void> {
        // leave a run without PostgreSQL
        const address = process.env["DESTACK_TEST_POSTGRES"] ?? "";
        if (address === "") {
            return;
        }

        // drop each schema of the run
        const session = postgres(address, { max: 1, fetch_types: false });
        const schemas = await session<{ name: string }[]>`
            SELECT nspname AS name FROM pg_namespace WHERE starts_with(nspname, ${`test_t${id}_`})`;
        for (const { name } of schemas) {
            await session.unsafe(`DROP SCHEMA "${name}" CASCADE`);
        }
        await session.end();
    },
};

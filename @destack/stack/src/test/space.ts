import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { databaseProvider } from "@destack/db";
import { BunSqliteDatabaseHost } from "@destack/db/bun";
import { onTestFinished } from "@destack/test";
import { database, SPACE_PROVIDER } from "../object/index.ts";
import type { SpaceOptions } from "../server/server.ts";

/** Keep a test space's main database as a SQLite file below a directory, as the space provider. */
export class TestSpaceDatabaseHost extends BunSqliteDatabaseHost {
    /** The provider code of the spaces' own databases. */
    override readonly provider: string = SPACE_PROVIDER;

    /** Provide spaces' own databases in a temporary directory removed once the test ends. */
    static async provider(): Promise<SpaceOptions["providers"][number]> {
        const directory = await mkdtemp(join(tmpdir(), "destack-stack-main-"));
        onTestFinished(() => rm(directory, { recursive: true, force: true }));

        return databaseProvider(new TestSpaceDatabaseHost(pathToFileURL(`${directory}/`)), database);
    }
}

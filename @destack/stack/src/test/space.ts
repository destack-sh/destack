import { pathToFileURL } from "node:url";
import { databaseProvider } from "@destack/db";
import type { Environment } from "@destack/environment";
import { BunSqliteDatabaseHost } from "@destack/db/bun";
import { database, SPACE_PROVIDER } from "../object/index.ts";
import type { SpaceOptions } from "../server/server.ts";

/** Store a test space's main database as a SQLite file below a directory, as the space provider. */
export class TestSpaceDatabaseHost extends BunSqliteDatabaseHost {
    /** The provider code of the spaces' own databases. */
    override readonly provider: string = SPACE_PROVIDER;

    /** Provide spaces' own databases in a world below a directory of the caller. */
    static over(
        directory: string,
        environment: Environment<"files" | "server">,
    ): SpaceOptions["providers"][number] {
        return databaseProvider(
            new TestSpaceDatabaseHost(pathToFileURL(`${directory}/`), environment),
            database,
        );
    }
}

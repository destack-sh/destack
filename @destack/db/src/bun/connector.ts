import type { DatabaseConnector } from "../declare/database.ts";

/** The connectors opening databases on Bun: SQLite files, loaded on first connect. */
export const connectors: Readonly<Record<string, DatabaseConnector>> = {
    sqlite: {
        code: "sqlite",
        connect: async (bound, declaration) => {
            const { bunSqliteConnector } = await import("./sqlite.ts");

            return bunSqliteConnector.connect(bound, declaration);
        },
    },
};

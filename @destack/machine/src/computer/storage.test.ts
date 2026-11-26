import { pathToFileURL } from "node:url";
import { BunSqliteDatabaseHost } from "@destack/db/bun";
import type { DatabaseCluster } from "@destack/db/database";
import { PostgresDatabaseHost } from "@destack/db/postgres";
import { TestDatabase } from "@destack/db/test";
import type { Environment } from "@destack/environment";
import { LocalKeyring, MemoryKeychain } from "@destack/identity";
import { mailConnector } from "@destack/mail/transport";
import { PrintTransport } from "@destack/mail/print";
import { Identifier } from "@destack/schema";
import { expect, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { LocalSpaceStorage } from "./storage.ts";

/** The spaces' databases as SQLite files below a directory, or as schemas of the test cluster. */
const BACKENDS = {
    sqlite: {
        databases: "files" as const,
        cluster: (directory: string, environment: Environment<"files" | "server">) =>
            new BunSqliteDatabaseHost(pathToFileURL(`${directory}/`), environment),
    },
    postgresql: {
        databases: new URL(TestDatabase.address()),
        cluster: (_directory: string, environment: Environment<"files" | "server">) =>
            new PostgresDatabaseHost(new URL(TestDatabase.address()), environment),
    },
};

test.each(Object.entries(BACKENDS))(
    "keep a space's own database in %s, listing it and reopening it at its log until deleted",
    async (provider, backend) => {
        // open the spaces' storage on the backend
        const environment = testHost();
        const directory = await environment.files.makeTemporaryDirectory("storage-");
        const keyring = await LocalKeyring.open(new MemoryKeychain(), "keys/storage", environment);
        await using storage = await LocalSpaceStorage.open({
            directory,
            endpoint: "http://127.0.0.1:1",
            keyring,
            environment,
            mail: mailConnector("print", new PrintTransport(() => undefined)),
            databases: backend.databases,
        });

        // open a space's volume, migrating its own database, then open it again
        const space = Identifier.create("space", environment);
        const volume = await storage.open(space);
        const position = await volume.database.log.position();
        await volume.close();
        const reopened = await storage.open(space);
        const reopenedAt = await reopened.database.log.position();
        await reopened.close();

        // list the space, then delete it with its own database
        await using cluster: DatabaseCluster = backend.cluster(directory, environment);
        const own = cluster.reference(space, space);
        const listed = await storage.list();
        const isStored = (await cluster.bytes(own)) > 0;
        await storage.delete(space);
        const remaining = await cluster.bytes(own).then(
            () => "kept",
            () => "dropped",
        );
        expect({
            provider: cluster.provider,
            listed: listed.includes(space),
            isSameLog: reopenedAt.log === position.log,
            isStored,
            deleted: (await storage.list()).includes(space),
            remaining,
        }).toEqual({
            provider,
            listed: true,
            isSameLog: true,
            isStored: true,
            deleted: false,
            remaining: "dropped",
        });

        await environment.files.remove(directory, { recursive: true });
    },
);

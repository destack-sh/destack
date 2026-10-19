import { classifyError, DatabaseError, type Insert } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { Identifier } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { repository } from "../src/object/index.ts";
import { homeDatabase } from "../src/test/home.ts";

test.each(TEST_DIALECTS)(
    "refuse repository rows that mix the columns of their origins on %s",
    async (dialect) => {
        // store the residency home's database
        const storage = await TestDatabase.create(dialect, homeDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        const scope = Identifier.create("space");
        const accountId = Identifier.create("account");

        // insert a row of an origin, reporting the code of a refusal
        const insert = async (origin: Partial<Insert<typeof repository.table>>) => {
            const now = Date.now();
            try {
                await storage.database.insert(repository.table).values({
                    id: Identifier.create("repository"),
                    scope,
                    accountId,
                    name: `site-${Identifier.create("space").slice(-8)}`,
                    createdAt: now,
                    updatedAt: now,
                    revision: 1,
                    hosting: "platform",
                    ...origin,
                });

                return "accepted";
            } catch (error) {
                const classified = classifyError(error);

                return classified instanceof DatabaseError ? classified.code : error;
            }
        };
        const remote = "https://git.example.test/site.git";
        const machineId = Identifier.create("machine");
        expect([
            await insert({
                hosting: "platform",
                provider: "local",
                providerRepositoryId: "r",
                remote,
            }),
            await insert({ hosting: "platform", remote }),
            await insert({ hosting: "git", remote, authentication: "anonymous" }),
            await insert({ hosting: "git", remote, authentication: "credential" }),
            await insert({
                hosting: "github",
                remote,
                connectionId: Identifier.create("connection"),
            }),
            await insert({ hosting: "github", remote }),
            await insert({ hosting: "machine", machineId, remote }),
            await insert({
                hosting: "git",
                remote,
                authentication: "anonymous",
                defaultReference: "main",
            }),
        ]).toEqual([
            "accepted",
            "INVALID_RECORD",
            "accepted",
            "accepted",
            "accepted",
            "INVALID_RECORD",
            "INVALID_RECORD",
            "INVALID_RECORD",
        ]);
    },
);

import { testSimulation } from "@destack/object/test";
import { classifyError, DatabaseError, type Insert } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Identifier } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { repository } from "../src/object/index.ts";

test("refuse repository rows that mix the columns of their origins", async () => {
    // store the repositories' table
    const simulation = testSimulation(0);
    const { clock } = simulation;
    const world = simulation.environment("test");
    const storage = await TestDatabase.create(repository.tables, {
        isMigrated: true,
        environment: world,
    });
    onTestFinished(() => storage.close());
    const scope = Identifier.create("space", world);
    const accountId = Identifier.create("account", world);

    // insert a row of an origin, reporting the code of a refusal
    const insert = async (origin: Partial<Insert<typeof repository.table>>) => {
        const now = clock.now();
        try {
            await storage.database.transaction(async (transaction) => {
                await transaction.insert(repository.table).values({
                    id: Identifier.create("repository", world),
                    scope,
                    accountId,
                    name: `site-${Identifier.create("space", world).slice(-8)}`,
                    createdAt: now,
                    updatedAt: now,
                    revision: 1,
                    hosting: "platform",
                    ...origin,
                });
            });

            return "accepted";
        } catch (error) {
            const classified = classifyError(error, storage.database.state.declared);

            return DatabaseError.is(classified) ? classified.reason : error;
        }
    };
    const remote = "https://git.example.test/site.git";
    const machineId = Identifier.create("machine", world);
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
            connectionId: Identifier.create("connection", world),
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
        "RECORD_INVALID",
        "accepted",
        "accepted",
        "accepted",
        "RECORD_INVALID",
        "RECORD_INVALID",
        "RECORD_INVALID",
    ]);
});

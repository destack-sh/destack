import { defineScenario } from "@destack/package/declare";
import { databaseInteraction } from "../scenario/interaction.ts";
import { BlobStore } from "./store.ts";

/** Store blobs the size of build modules and fetch them back. */
export const storeAndFetchModules = defineScenario({
    of: BlobStore,
    interaction: databaseInteraction,
    name: "store-and-fetch-modules",
    description: "store blobs the size of build modules and fetch them back",
    given: { examples: [] },
    parameters: { bytes: 65_536, times: 100, warmup: 1 },
    when: [
        {
            operation: "blob.write",
            times: "{times}",
            warmup: "{warmup}",
            step: { action: "write", bytes: "{bytes}" },
            then: [
                { metric: { name: "db.client.operation.duration" }, atMost: 4 },
                { metric: { name: "destack.transaction.run.duration" }, atMost: 2 },
            ],
        },
        {
            operation: "blob.fetch",
            times: "{times}",
            step: { action: "fetch" },
            then: [{ metric: { name: "db.client.operation.duration" }, atMost: 1 }],
        },
    ],
    then: [{ observe: { count: "blobs" }, is: "{warmup + times}" }],
});

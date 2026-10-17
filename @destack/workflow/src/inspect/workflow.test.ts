import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { defineWorkflow } from "../workflow/workflow.ts";
import { describeWorkflow } from "./workflow.ts";

/** Send each person one digest a day, at most ten a minute and one at a time per person. */
const digest = defineWorkflow({
    name: "digest",
    input: schema.object({ person: schema.string() }),
    output: schema.object({ sent: schema.number() }),
    retries: { maximumAttempts: 3 },
    timeout: { hours: 1 },
    expiring: { after: { days: 7 } },
    limits: {
        rate: { count: 10, every: { minutes: 1 } },
        concurrency: { slots: 1 },
        key: { input: "person" },
    },
    singleton: true,
    triggers: {
        daily: {
            kind: "schedule",
            schedule: {
                timing: { timing: "cron", cron: "0 7 * * *", timezone: "Europe/Zurich" },
                concurrency: "forbid",
                deadline: 60_000,
            },
            input: () => ({ person: "everyone" }),
        },
    },
});

/** Count the runs of each caller, unlimited otherwise. */
const tally = defineWorkflow({
    name: "tally",
    input: schema.object({}),
    limits: { rate: { count: 1, every: { seconds: 1 }, burst: 5 }, key: "caller" },
});

test("describe a workflow's schemas, retries, timeout, expiry, limits and triggers, and a caller's limit", () => {
    expect([describeWorkflow(digest), describeWorkflow(tally).limits]).toEqual([
        {
            name: "digest",
            input: {
                $schema: "https://json-schema.org/draft/2020-12/schema",
                type: "object",
                properties: { person: { type: "string" } },
                required: ["person"],
                additionalProperties: false,
            },
            output: {
                $schema: "https://json-schema.org/draft/2020-12/schema",
                type: "object",
                properties: { sent: { type: "number" } },
                required: ["sent"],
                additionalProperties: false,
            },
            waits: [
                {
                    packageId: "package-01a1441c-7a6d-7444-b168-a063d720a275",
                    name: "change-request",
                },
            ],
            retries: {
                initialInterval: 1000,
                backoffCoefficient: 2,
                maximumInterval: 100_000,
                maximumAttempts: 3,
            },
            timeout: 3_600_000,
            expiring: 604_800_000,
            limits: {
                rate: { count: 10, every: 60_000, burst: 10 },
                concurrency: { slots: 1 },
                key: { input: "person" },
            },
            isSingleton: true,
            triggers: [
                {
                    name: "daily",
                    kind: "schedule",
                    schedule: {
                        timing: { timing: "cron", cron: "0 7 * * *", timezone: "Europe/Zurich" },
                        concurrency: "forbid",
                        deadline: 60_000,
                    },
                },
            ],
        },
        { rate: { count: 1, every: 1000, burst: 5 }, key: "caller" },
    ]);
});

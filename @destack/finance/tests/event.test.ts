import { asc, eq } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import { aligned } from "@destack/schema";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { meterEvent } from "../src/object/index.ts";
import { Finance, ids } from "./fixture/finance.ts";
import { requests, storage } from "./fixture/storage.ts";

test.each(TEST_DIALECTS)(
    "keep a meter event once per CloudEvents source and id, refusing a repeat of the pair on %s",
    async (dialect) => {
        const finance = await Finance.open(dialect);
        onTestFinished(() => finance.close());

        // append an event and the same id from another source, refusing a repeat of either pair
        const event = {
            eventId: "request-1",
            packageId: storage.id,
            meter: requests.name,
            value: 3,
            time: Date.UTC(2026, 9, 2),
            source: "/cells/eu-1",
        };
        const append = (input: typeof event) =>
            finance.server.executeAsSystem(
                meterEvent,
                "create",
                [{ scope: ids.buyer, input }],
                Date.now(),
            );
        const first = aligned(await append(event), 0);
        const other = aligned(await append({ ...event, source: "/cells/eu-2" }), 0);
        expect([
            await refusal(append(event)),
            await refusal(append({ ...event, value: 4 })),
        ]).toEqual([
            ["DUPLICATE", "a record with the same unique key exists"],
            ["DUPLICATE", "a record with the same unique key exists"],
        ]);

        // keep the two events alone
        const kept = await finance.test.database
            .select()
            .from(meterEvent.table)
            .where(eq(meterEvent.table.scope, ids.buyer))
            .orderBy(asc(meterEvent.table.source));
        expect(kept).toEqual([first, other]);
    },
);

import { TEST_DIALECTS } from "@destack/db/test";
import { expect, onTestFinished, test } from "@destack/test";
import { usage } from "../src/meter/index.ts";
import { Finance, ids } from "./fixture/finance.ts";
import { requests, storage } from "./fixture/storage.ts";

test.each(TEST_DIALECTS)(
    "keep a use routed to an account once per source and identity, a repeat changing nothing, on %s",
    async (dialect) => {
        const finance = await Finance.open(dialect);
        onTestFinished(() => finance.close());

        // route a use, the same identity from another source, and the first again with another quantity
        const use = {
            id: "request-1",
            source: "/cells/eu-1",
            meter: { packageId: storage.id, name: requests.name },
            quantity: 3,
            time: Date.UTC(2026, 9, 2),
        };
        await finance.fixture.use(ids.buyer, [use, { ...use, source: "/cells/eu-2" }]);
        await finance.fixture.use(ids.buyer, [{ ...use, quantity: 4 }]);

        // keep the two uses alone, the first as it first came
        const { events } = await finance.fixture.events.query(usage, { scope: ids.buyer });
        expect(events.map((event) => [event.source, event.id, event.keys.quantity])).toEqual([
            ["/cells/eu-1", "request-1", 3],
            ["/cells/eu-2", "request-1", 3],
        ]);
    },
);

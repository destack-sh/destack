import { Snapshot } from "@destack/db";
import { placement, spaceShape } from "@destack/account/object";
import { present, schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { QUERIES_PUBLICATION } from "@destack/object";
import { expect, test } from "@destack/test";
import { serveSpace } from "./server.ts";
import { ids, placeSpace, StackFixture } from "../test/index.ts";

/** The queries a universe replica requests, by name. */
const UniverseQueries = schema.object({
    queries: schema.record(schema.string(), schema.looseObject({ object: schema.string() })),
});

test("replicate the served space's own placement alone, leaving placements out of every universe replica", async () => {
    // serve a placed space
    const fixture = await StackFixture.open();
    await placeSpace(fixture.database);
    const { store } = serveSpace(await fixture.options());

    // list the replicas the space's database keeps, and the types its universe queries request
    const listed = await present(store.subscriber, "the space's subscriber").subscriptions(
        Snapshot.live(store.database),
    );
    const subscriptions = listed.map((entry) => entry.subscription);
    const own = subscriptions.filter((each) => each.name.startsWith(`${spaceShape.name}:`));
    const universes = subscriptions.filter((each) =>
        each.name.startsWith(`${QUERIES_PUBLICATION}:${ids.space}:`),
    );
    const requested = universes.some((each) =>
        Object.values(UniverseQueries.parse(each.parameters).queries).some(
            (query) => query.object === placement.name,
        ),
    );

    // follow the space's placement by its identifier, and the space's universe rows without placements
    expect({ own, requested }).toEqual({
        own: [spaceShape.subscription({ scope: Scope.universe.id, space: ids.space }, ids.space)],
        requested: false,
    });
});

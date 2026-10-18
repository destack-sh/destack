import { UNIVERSE_PUBLICATION } from "@destack/access";
import { placement, spaceShape } from "@destack/account/object";
import { present, schema } from "@destack/schema";
import { ObjectTypeReference, Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import { serveSpace } from "./server.ts";
import { ids, placeSpace, StackFixture } from "../test/index.ts";

/** The rows a universe replica requests, by type. */
const UniverseRows = schema.object({
    rows: schema.array(schema.looseObject({ type: ObjectTypeReference })),
});

test("replicate the served space's own placement alone, leaving placements out of every universe replica", async () => {
    // serve a placed space
    const fixture = await StackFixture.open();
    await placeSpace(fixture.database);
    const { store } = serveSpace(await fixture.options());

    // list the replicas the space's database keeps, and the types its universe replicates request
    const listed = await present(store.subscriber?.subscriptions, "the space's subscriptions")();
    const subscriptions = listed.map((entry) => entry.subscription);
    const own = subscriptions.filter((each) => each.name.startsWith(`${spaceShape.name}:`));
    const universes = subscriptions.filter((each) => each.publication === UNIVERSE_PUBLICATION);
    const requested = universes.map((each) =>
        UniverseRows.parse(each.parameters).rows.some(
            (row) =>
                ObjectTypeReference.key(row.type) ===
                ObjectTypeReference.key(placement.typeReference),
        ),
    );

    // follow the space's placement by its identifier, and the space's universe rows without placements
    expect({ own, requested }).toEqual({
        own: [spaceShape.subscription({ scope: Scope.universe.id, space: ids.space }, ids.space)],
        requested: [false],
    });
});

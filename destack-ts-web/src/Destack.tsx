import { ReactiveSession, SessionProvider } from "@destack-web/language";
import { IndexedDBStore } from "@destack-web/store";
import {
  ACTIVE_SESSION,
  ACTIVE_SPACE,
  Layer,
  LineShape,
  Region,
  Space,
  SpaceStatus,
  StoreKey,
} from "destack";
import React from "react";
import LayerView from "./Layer";

const store = new IndexedDBStore({
  types: [StoreKey.ENTITY_PRIMARY],
});
await store.open();
const session = new ReactiveSession({ store });
ACTIVE_SESSION.set(session);

let space = await Space.get({ where: Space.property("slug").eq("my-space") }).executeOneOrNone();
let layer: Layer;
if (space == null) {
  space = new Space({
    name: "My Space",
    slug: "my-space",
    status: SpaceStatus.ACTIVE,
    region: Region.ZURICH,
  });
  session.create(space);
  layer = new Layer({ name: "My Layer", space });
  session.create(layer);
  await session.commit();
} else {
  layer = await Layer.get({
    where: Layer.property("space").eq(space),
    Lines: LineShape.search(),
  }).executeOne();
}
ACTIVE_SPACE.set(space);

const Destack: React.FC = () => {
  return (
    <SessionProvider session={session}>
      <div>
        <LayerView layerPtr={layer.toRef()} />
      </div>
    </SessionProvider>
  );
};

export default Destack;

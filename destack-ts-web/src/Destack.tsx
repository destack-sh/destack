import { ReactiveSession, SessionProvider } from "@destack-web/language";
import { IndexedDBStore } from "@destack-web/store";
import { ACTIVE_SESSION, Canvas, LineShape, Region, Space, SpaceStatus, StoreKey } from "destack";
import React from "react";
import CanvasView from "./Canvas";

const store = new IndexedDBStore({
  types: [StoreKey.GLOBAL_ENTITY_PRIMARY, StoreKey.SPATIAL_ENTITY_PRIMARY],
});
await store.open();
const session = new ReactiveSession({ store });
ACTIVE_SESSION.set(session);

let space = await Space.get({ where: Space.property("slug").eq("my-space") }).executeOneOrNone();
let canvas: Canvas;
if (space == null) {
  space = new Space({
    name: "My Space",
    slug: "my-space",
    status: SpaceStatus.ACTIVE,
    region: Region.ZURICH,
  });
  session.create(space);
  canvas = new Canvas({ name: "My Canvas", space });
  session.create(canvas);
  await session.commit();
} else {
  canvas = await Canvas.get({
    where: Canvas.property("space").eq(space),
    Lines: LineShape.search(),
  }).executeOne();
}

const Destack: React.FC = () => {
  return (
    <SessionProvider session={session}>
      <div>
        <CanvasView canvasPtr={canvas.toRef()} />
      </div>
    </SessionProvider>
  );
};

export default Destack;

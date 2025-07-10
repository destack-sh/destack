import { ReactiveSession, SessionProvider } from "@destack-web/language";
import { ACTIVE_SESSION, Canvas, MemoryStore, Region, Space, SpaceStatus, StoreKey } from "destack";
import React from "react";
import CanvasView from "./Canvas";

const store = new MemoryStore({
  types: [StoreKey.GLOBAL_ENTITY_PRIMARY, StoreKey.SPATIAL_ENTITY_PRIMARY],
});
const session = new ReactiveSession({ store });
ACTIVE_SESSION.set(session);

const space = new Space({
  name: "My Space",
  slug: "my-space",
  status: SpaceStatus.ACTIVE,
  region: Region.ZURICH,
});
session.create(space);
const canvas = new Canvas({ name: "My Canvas", space });
session.create(canvas);
await session.commit();

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

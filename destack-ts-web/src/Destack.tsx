import { ReactiveSession } from "@destack-web/language";
import { createStore, Provider } from "jotai";
import { ACTIVE_SESSION, Canvas, MemoryStore, Region, Space, SpaceStatus, StoreKey } from "destack";
import React from "react";
import CanvasView from "./Canvas";

const store = new MemoryStore({
  types: [StoreKey.GLOBAL_ENTITY_PRIMARY, StoreKey.SPATIAL_ENTITY_PRIMARY],
});
const atomStore = createStore();
const session = new ReactiveSession({ store, atomStore });
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
    <Provider store={atomStore}>
      <div>
        <CanvasView canvasPtr={canvas.toRef()} />
      </div>
    </Provider>
  );
};

export default Destack;

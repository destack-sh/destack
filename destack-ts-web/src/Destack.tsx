import { ReactiveSession, SessionProvider } from "@destack-web/language";
import {
  ACTIVE_BRANCH,
  ACTIVE_SESSION,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  createSpace,
  Layer,
  MemoryStore,
  StoreKey,
} from "destack";
import React from "react";
import LayerView from "./Layer";

const store = new MemoryStore({
  keys: [StoreKey.ENTITY_PRIMARY],
});
await store.open();
const session = new ReactiveSession({ store, epoch: 0 });
ACTIVE_SESSION.set(session);

// create new space
const { space, branch, snapshot } = createSpace({ session });
ACTIVE_SPACE.set(space);
ACTIVE_BRANCH.set(branch);
ACTIVE_SNAPSHOT.set(snapshot);
const layer = new Layer({ name: "My Layer" });
session.create(layer);
await session.commit();

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

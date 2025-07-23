import { ReactiveSession, SessionProvider } from "@destack-web/language";
import {
  ACTIVE_BRANCH,
  ACTIVE_SESSION,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  createSpace,
  Layer,
  MemoryGraph,
} from "destack";
import React from "react";
import LayerView from "./Layer";

const graph = new MemoryGraph();
await graph.open();
const session = new ReactiveSession({ graph, epoch: 0 });
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
      <LayerView layerPtr={layer.toRef()} />
    </SessionProvider>
  );
};

export default Destack;

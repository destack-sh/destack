import { ReactiveSession, SessionProvider } from "@destack-web/language";
import {
  ACTIVE_BRANCH,
  ACTIVE_SESSION,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  Layer,
  MemoryGraph,
  Region,
  Space,
  Universe,
  uuid4,
} from "destack";
import React from "react";
import LayerView from "./Layer";

const graph = new MemoryGraph();
await graph.open();
const session = new ReactiveSession({
  graph,
  epoch: 0,
  actor: Universe.ACTOR,
  client: Universe.CLIENT,
  clientNonce: uuid4(),
});
ACTIVE_SESSION.set(session);

// create new space
const { space, rootBranch, headSnapshot } = Space.createSpace({
  session,
  name: "My Space",
  slug: "my-space",
  region: Region.ZURICH,
  ownedBy: session.actorPtr,
});
ACTIVE_SPACE.set(space);
ACTIVE_BRANCH.set(rootBranch);
ACTIVE_SNAPSHOT.set(headSnapshot);
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

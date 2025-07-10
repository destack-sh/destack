import { ReactiveSession, ReactiveSupergraph } from "@destack-web/language";
import { ACTIVE_SESSION, MemoryStore, StoreKey } from "destack";
import React from "react";
import Canvas from "./Canvas";

const store = new MemoryStore({
  types: [StoreKey.GLOBAL_ENTITY_PRIMARY, StoreKey.SPATIAL_ENTITY_PRIMARY],
});
const session = new ReactiveSession({ store });
ACTIVE_SESSION.set(session);

const Destack: React.FC = () => {
  return (
    <div>
      <Canvas />
    </div>
  );
};

export default Destack;

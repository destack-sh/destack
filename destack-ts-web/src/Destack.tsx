import { ACTIVE_SESSION, MemoryEntityStore, Session, StoreKey } from "destack";
import React from "react";
import Canvas from "./Canvas";

const store = new MemoryEntityStore({
  types: [StoreKey.GLOBAL_ENTITY_PRIMARY, StoreKey.SPATIAL_ENTITY_PRIMARY],
});
const session = new Session({ store });
ACTIVE_SESSION.set(session);

const Destack: React.FC = () => {
  return (
    <div>
      <Canvas />
    </div>
  );
};

export default Destack;

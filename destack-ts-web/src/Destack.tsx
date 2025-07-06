import { ACTIVE_SESSION, MemoryStore, Session, StoreType } from "destack";
import React from "react";
import Canvas from "./Canvas";

const store = new MemoryStore({
  types: [StoreType.GLOBAL_ENTITY_PRIMARY, StoreType.SPATIAL_ENTITY_PRIMARY],
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

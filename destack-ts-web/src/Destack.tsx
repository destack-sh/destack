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
import type { FunctionComponent } from "preact";
import { useEffect, useState } from "preact/hooks";
import LayerView from "./Layer";

const graph = new MemoryGraph();
// await graph.open();
const session = new ReactiveSession({
  graph,
  remoteEpoch: 0,
  localEpoch: 0,
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
// await session.commit();

const PerformanceIndicator: FunctionComponent = () => {
  const [fps, setFps] = useState(0);
  const [memoryUsage, setMemoryUsage] = useState(0);

  useEffect(() => {
    let frameCount = 0;
    let lastTime = performance.now();
    let animationId: number;

    const updateStats = () => {
      frameCount++;
      const currentTime = performance.now();

      // update fps every second
      if (currentTime - lastTime >= 1000) {
        setFps(Math.round((frameCount * 1000) / (currentTime - lastTime)));
        frameCount = 0;
        lastTime = currentTime;

        // update memory usage if available
        if ("memory" in performance) {
          const memory = (performance as any).memory;
          setMemoryUsage(Math.round(memory.usedJSHeapSize / 1024 / 1024));
        }
      }

      animationId = requestAnimationFrame(updateStats);
    };

    animationId = requestAnimationFrame(updateStats);

    return () => {
      cancelAnimationFrame(animationId);
    };
  }, []);

  return (
    <div
      style={{
        position: "fixed",
        top: "10px",
        right: "10px",
        background: "rgba(0, 0, 0, 0.8)",
        color: "white",
        padding: "8px 12px",
        borderRadius: "4px",
        fontFamily: "monospace",
        fontSize: "12px",
        zIndex: 9999,
      }}
    >
      <div>FPS: {fps}</div>
      {memoryUsage > 0 && <div>Memory: {memoryUsage} MB</div>}
    </div>
  );
};

const Destack: FunctionComponent = () => {
  return (
    <SessionProvider session={session}>
      <LayerView layerPtr={layer.toRef()} />
      <PerformanceIndicator />
    </SessionProvider>
  );
};

export default Destack;

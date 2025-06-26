import { signal } from "@preact/signals-react";
import { useSignals } from "@preact/signals-react/runtime";
import { ACTIVE_SESSION, Canvas, LineShape, NODE_DEFINITIONS, Session } from "destack";

import { AnimatePresence, motion } from "motion/react";

const session = new Session({});
ACTIVE_SESSION.set(session);

const canvas = new Canvas({
  name: "My Canvas",
});
const lines = canvas.getChildren(LineShape);
const count = signal(0);
const isVisible = signal(true);

const Space: React.FC = () => {
  useSignals();
  return (
    <div>
      <h1>Hello World!</h1>
      <button onClick={() => count.value++}>Click me</button>
      <p>Count: {count.value}</p>
      <p>{NODE_DEFINITIONS.length}</p>

      <div style={container}>
        <AnimatePresence initial={false}>
          {isVisible.value ? (
            <motion.div
              initial={{ opacity: 0, scale: 0 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0 }}
              style={box}
              key="box"
            />
          ) : null}
        </AnimatePresence>
        <motion.button
          style={button}
          onClick={() => {
            isVisible.value = !isVisible.value;
          }}
          whileTap={{ y: -5 }}
        >
          {isVisible.value ? "Hide" : "Show"}
        </motion.button>
      </div>
    </div>
  );
};

/**
 * ==============   Styles   ================
 */

const container: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  width: 100,
  height: 160,
  position: "relative",
};

const box: React.CSSProperties = {
  width: 100,
  height: 100,
  backgroundColor: "#0cdcf7",
  borderRadius: "10px",
};

const button: React.CSSProperties = {
  backgroundColor: "#0cdcf7",
  borderRadius: "10px",
  padding: "10px 20px",
  color: "#0f1115",
  position: "absolute",
  bottom: 0,
  left: 0,
  right: 0,
};

export default Space;

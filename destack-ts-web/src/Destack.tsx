import { signal } from "@preact/signals-react";
import { ACTIVE_SESSION, Canvas, LineShape, NODE_DEFINITIONS, Session } from "destack";

const session = new Session({});
ACTIVE_SESSION.set(session);

const canvas = new Canvas({
  name: "My Canvas",
});
const lines = canvas.getChildren(LineShape);
const count = signal(0);
const isVisible = signal(true);

const Destack: React.FC = () => {
  return (
    <div>
      <h1>Hello World!</h1>
      <button onClick={() => count.value++}>Click me</button>
      <p>Count: {count.value}</p>
      <p>{NODE_DEFINITIONS.length}</p>
    </div>
  );
};

export default Destack;

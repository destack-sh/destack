import { ACTIVE_SESSION, activeSession, Canvas, LineShape, NodeType, Session } from "destack";
import { signal } from "@preact/signals-react";

const session = new Session({});
ACTIVE_SESSION.set(session);
const count = signal(0);

const canvas = new Canvas({
  name: "My Canvas",
});
const lines = canvas.getChildren(LineShape);
const Space: React.FC = () => {
  return (
    <div>
      <h1>Hello World!</h1>
      <button onClick={() => count.value++}>Click me</button>
      <p>Count: {count}</p>
    </div>
  );
};

export default Space;

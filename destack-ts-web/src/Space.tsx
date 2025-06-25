import { ACTIVE_SESSION, activeSession, Canvas, HasName, HasSlug, NodeType, Session } from "destack";

const session = new Session({});
ACTIVE_SESSION.set(session);

console.log(ACTIVE_SESSION.get());
console.log(activeSession());
const canvas = new Canvas({
  name: "My Canvas",
});
const lines = canvas.getChildren({ nodeType: NodeType.LINE_SHAPE });
const Space: React.FC = () => {
  return (
    <div>
      <h1>Hello World!</h1>
    </div>
  );
};

export default Space;

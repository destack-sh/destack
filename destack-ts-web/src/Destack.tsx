import { ACTIVE_SESSION, Session } from "destack";
import Canvas from "./Canvas";

const session = new Session({});
ACTIVE_SESSION.set(session);

const Destack: React.FC = () => {
  return (
    <div>
      <Canvas />
    </div>
  );
};

export default Destack;

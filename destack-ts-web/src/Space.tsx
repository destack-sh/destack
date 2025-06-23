import { Canvas, Layer, NodeType } from "destack";

const canvas = new Canvas({
  name: "My Canvas",
});

const lines = canvas.getChildren(NodeType.LINE)

const Space: React.FC = () => {
  return (
    <div>
      <h1>Hello World</h1>
    </div>
  );
};

export default Space;

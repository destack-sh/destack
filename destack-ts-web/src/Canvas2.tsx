import { Canvas, LineShape, NodeReference } from "destack";

export const Canvas2: React.FC = (props: { canvas: Canvas | NodeReference }) => {
  const { roots: lines } = useQuery({
    query: LineShape.search({
      where: LineShape.property("parent").eq(props.canvas),
    }),
  });

  return <div>hey</div>;
};

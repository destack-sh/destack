import { Canvas, LineShape, NodeReference } from "destack";

// nocheckin: reactive TS Store/Queries/Edits
export const Canvas2: React.FC = (props: { canvas: Canvas | NodeReference }) => {
  const { roots: lines } = useQuery({
    query: LineShape.search({
      where: LineShape.property("parent").eq(props.canvas),
    }),
  });

  return <div>hey</div>;
};

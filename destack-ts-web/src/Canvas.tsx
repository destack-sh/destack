import { renderStroke } from "@destack-web/shared/freehand/svg";
import {
  activeSession,
  Canvas,
  Easing,
  LineShape,
  NodeReference,
  PointerMoveEvent,
  Stroke,
  StrokeType,
  Vector2f,
} from "destack";
import { atom } from "jotai";
import React, { useRef, useState } from "react";

const currentLineAtom = atom<LineShape | null>(null);

// nocheckin: reactive TS graphs & querying
// basic reactive keys (in (Reactive)Graphs):
//  - get: snapshot_id + node_id
//  - get_children: snapshot_id + node_id + [node_type]

const strokeOptions = new Stroke({
  type: StrokeType.FREEHAND,
  size: 12,
  thinning: 0.5,
  smoothing: 0.62,
  streamline: 0.25,
  easing: Easing.LINEAR,
});

export const CanvasView: React.FC<{ canvasPtr: NodeReference }> = ({ canvasPtr }) => {
  console.log("CanvasView.render", canvasPtr.id);
  const session = activeSession();
  const canvas = session.supergraph.getOrError(canvasPtr.id) as Canvas;
  // const lines = useAtomValue(atom(() => canvas.getChildren(LineShape)));
  const lines = [];

  const [isDrawing, setIsDrawing] = useState(false);
  const [currentLine, setCurrentLine] = useState<LineShape | null>(null);

  const svgRef = useRef<SVGSVGElement>(null);

  const getMousePosition = (event: React.MouseEvent<SVGSVGElement>): Vector2f => {
    if (!svgRef.current) {
      throw new Error("SVG element not found");
    }
    const rect = svgRef.current.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    return new Vector2f({ x, y });
  };

  // begin drawing on mouse down
  const handleMouseDown = (event: React.MouseEvent<SVGSVGElement>) => {
    const point = getMousePosition(event);
    setIsDrawing(true);
    const newLine = new LineShape({ name: "LineShape", points: [point] });
    setCurrentLine(newLine);
    canvas.addChild(newLine);
    console.log("handleMouseDown", canvas.getChildren().length);
    session.commit();
  };

  // add points on mouse move
  const handleMouseMove = (event: React.MouseEvent<SVGSVGElement>) => {
    const currentPoint = getMousePosition(event);

    if (isDrawing && currentLine) {
      const lastPoint = currentLine.points[currentLine.points.length - 1];
      if (lastPoint.x !== currentPoint.x || lastPoint.y !== currentPoint.y) {
        currentLine.points = [...currentLine.points, currentPoint];
      }
    }
    const mouseEvent = new PointerMoveEvent({
      position: currentPoint,
      shiftKey: event.shiftKey,
      ctrlKey: event.ctrlKey,
      altKey: event.altKey,
      metaKey: event.metaKey,
    });
    session.append(mouseEvent);
    session.commit();
  };

  // finish drawing on mouse up
  const handleMouseUp = () => {
    if (isDrawing && currentLine) {
      setIsDrawing(false);
      setCurrentLine(null);
    }
  };

  return (
    <div style={{ width: "100vw", height: "100vh", display: "flex", flexDirection: "column" }}>
      {/* Stats */}
      <div
        style={{
          height: "100px",
          overflow: "auto",
          background: "gray",
          display: "flex",
          flexDirection: "column",
        }}
      >
        <span>Lines: {lines.length}</span>
      </div>

      {/* Lines */}
      <svg
        ref={svgRef}
        width="100%"
        height="100%"
        style={{
          flex: 1,
          background: "white",
          cursor: "crosshair",
          border: "none",
        }}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
      >
        {/* render completed lines */}
        {lines.map((line, index) => (
          <g key={index}>
            <path
              d={renderStroke(line.points, strokeOptions, { isComplete: true })}
              fill="#2563eb"
              stroke="none"
            />
          </g>
        ))}
      </svg>
    </div>
  );
};

export default CanvasView;

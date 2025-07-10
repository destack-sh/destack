import { renderStroke } from "@destack-web/shared/freehand/svg";
import { Signal, signal, useComputed, useSignal, useSignalEffect } from "@preact/signals-react";
import {
  activeSession,
  Easing,
  Event,
  LineShape,
  Node,
  NodeReference,
  PointerMoveEvent,
  Query,
  QueryConnection,
  Stroke,
  StrokeType,
  Vector2f,
} from "destack";
import React, { useRef } from "react";

const currentLine = signal<LineShape | null>(null);

// nocheckin: reactive TS graphs & querying
// basic reactive keys (in (Reactive)Graphs):
//  - get: snapshot_id + node_id
//  - get_children: snapshot_id + node_id + [node_type]

function useQuery<T extends Node = Node>(
  query: Signal<Query<T>>,
): { connection: Signal<QueryConnection<T> | null>; nodes: Signal<readonly T[]> } {
  const session = activeSession();
  const connection: Signal<QueryConnection<T> | null> = useSignal(null);
  const nodes = useComputed(() => connection.value?.toList() ?? []);

  useSignalEffect(() => {
    query.value.execute().then((c) => {
      connection.value = c;
      console.log("query.execute", query.value.name, c.nodes.length, {
        store: c.store,
        storeRepr: c.store.repr(),
        query: query.value,
      });
    });
  });

  return { connection, nodes };
}

export const CanvasView: React.FC<{ canvasPtr: NodeReference }> = ({ canvasPtr }) => {
  const session = activeSession();
  const canvas = session.supergraph.get(canvasPtr.id);
  const lineQuery = useSignal(LineShape.search({}));
  const eventQuery = useSignal(Event.search({}));
  const { nodes: lines } = useQuery(lineQuery);
  const { nodes: events } = useQuery(eventQuery);

  const isDrawing = useSignal(false);
  const lastMousePosition = useSignal<Vector2f | null>(null);
  const strokeOptions = new Stroke({
    type: StrokeType.FREEHAND,
    size: 12,
    thinning: 0.5,
    smoothing: 0.62,
    streamline: 0.25,
    easing: Easing.LINEAR,
  });

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
    isDrawing.value = true;
    lastMousePosition.value = point;
    currentLine.value = new LineShape({ name: "LineShape", points: [point] });
    session.create(currentLine.value);
    session.commit();
  };

  // add points on mouse move
  const handleMouseMove = (event: React.MouseEvent<SVGSVGElement>) => {
    const currentPoint = getMousePosition(event);

    if (isDrawing.value && currentLine.value) {
      const lastPoint = currentLine.value.points[currentLine.value.points.length - 1];
      if (lastPoint.x !== currentPoint.x || lastPoint.y !== currentPoint.y) {
        currentLine.value.points = [...currentLine.value.points, currentPoint];
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
    if (isDrawing.value && currentLine.value) {
      isDrawing.value = false;
      currentLine.value = null;
      lastMousePosition.value = null;
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
        <span>Lines: {lines.value.length}</span>
        <span>Events: {events.value.length}</span>
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
        {lines.value.map((line, index) => (
          <g key={index}>
            <path
              d={renderStroke(line.points, strokeOptions, { isComplete: true })}
              fill="#2563eb"
              stroke="none"
            />
          </g>
        ))}
      </svg>

      {/* Events */}
      <div style={{ height: "100px", overflow: "auto", background: "gray" }}>
        {/* render events */}
        {events.value.slice(-3).map((event, index) => (
          <div key={index}>{event.repr()}</div>
        ))}
      </div>
    </div>
  );
};

export default CanvasView;

import { signal } from "@preact/signals-react";
import { Vector3 } from "destack";
import React from "react";
import { svgInk } from "@destack-web/shared/freehand/svg";
import { StrokeOptions } from "@destack-web/shared/freehand/types";

type Line = {
  points: Vector3[];
};

const currentLine = signal<Line | null>(null);

export const Canvas: React.FC = () => {
  const lines = signal<Line[]>([]);

  // create example stroke with curved path
  const examplePoints: Vector3[] = [
    new Vector3({x: 50, y: 100, z: 0.8}),
    new Vector3({ x: 100, y: 80, z: 0.9 }),
    new Vector3({ x: 150, y: 90, z: 0.7 }),
    new Vector3({ x: 200, y: 60, z: 0.8 }),
    new Vector3({ x: 250, y: 70, z: 0.6 }),
    new Vector3({ x: 300, y: 50, z: 0.9 }),
    new Vector3({ x: 350, y: 80, z: 0.7 }),
    new Vector3({ x: 400, y: 100, z: 0.8 }),
  ];

  const strokeOptions: StrokeOptions = {
    size: 16,
    thinning: 0.5,
    smoothing: 0.5,
    streamline: 0.5,
    simulatePressure: false,
    start: {
      cap: true,
      taper: true,
    },
    end: {
      cap: true,
      taper: true,
    },
  };

  const svgPath = svgInk(examplePoints, strokeOptions);

  return (
    <div style={{ padding: "20px" }}>
      <h2>Canvas with Example Stroke</h2>
      <svg
        width="500"
        height="200"
        viewBox="0 0 500 200"
        style={{ border: "1px solid #ccc", background: "white" }}
      >
        <path
          d={svgPath}
          fill="#2563eb"
          stroke="none"
        />
      </svg>
    </div>
  );
};
export default Canvas;

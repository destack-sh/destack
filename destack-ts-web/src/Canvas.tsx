import { renderStroke } from "@destack-web/shared/freehand/svg";
import { computed, Signal, signal } from "@preact/signals-react";
import { Easing, Line, Stroke, StrokeCap, StrokeType, Vector2 } from "destack";
import React, { useRef } from "react";

// nocheckin
const 

const size = signal(12);
const thinning = signal(0.5);
const smoothing = signal(0.6);
const streamline = signal(0.6);
const simulatePressure = signal(true);
const taperStart = signal(false);
const taperEnd = signal(false);
const isDrawing = signal(false);
const lastMousePosition = signal<Vector2 | null>(null);
const strokeOptions: Signal<Stroke> = computed(
  () =>
    new Stroke({
      type: StrokeType.FREEHAND,
      size: size.value,
      thinning: thinning.value,
      smoothing: smoothing.value,
      streamline: streamline.value,
      simulatePressure: simulatePressure.value,
      easing: Easing.LINEAR,
      start: new StrokeCap({
        cap: true,
        taper: taperStart.value,
        easing: Easing.EASE_OUT_CUBIC,
      }),
      end: new StrokeCap({
        cap: true,
        taper: taperEnd.value,
        easing: Easing.EASE_OUT_CUBIC,
      }),
    }),
);

const currentLine = signal<Line | null>(null);
const lines = signal<Line[]>([]);
const selectedLine = signal<Line | null>(null);

export const Canvas: React.FC = () => {
  const svgRef = useRef<SVGSVGElement>(null);

  const getMousePosition = (event: React.MouseEvent<SVGSVGElement>): Vector2 => {
    if (!svgRef.current) {
      throw new Error("SVG element not found");
    }
    const rect = svgRef.current.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    return new Vector2({ x, y });
  };

  // begin drawing on mouse down
  const handleMouseDown = (event: React.MouseEvent<SVGSVGElement>) => {
    isDrawing.value = true;
    const point = getMousePosition(event);
    lastMousePosition.value = point;
    currentLine.value = new Line({ points: [point] });
    console.log("mouse down", currentLine.value.points.length);
  };

  // add points on mouse move
  const handleMouseMove = (event: React.MouseEvent<SVGSVGElement>) => {
    if (!isDrawing.value || !currentLine.value) return;
    const currentPoint = getMousePosition(event);
    const lastPoint = currentLine.value.points[currentLine.value.points.length - 1];
    if (lastPoint.x !== currentPoint.x || lastPoint.y !== currentPoint.y) {
      currentLine.value = new Line({
        points: [...currentLine.value.points, currentPoint],
      });
      console.log("mouse move", currentLine.value.points.length, currentPoint.repr());
    }
  };

  // finish drawing on mouse up
  const handleMouseUp = () => {
    if (!isDrawing.value || !currentLine.value) return;
    isDrawing.value = false;
    lines.value = [...lines.value, currentLine.value];
    currentLine.value = null;
    lastMousePosition.value = null;
    console.log("mouse up", lines.value[lines.value.length - 1].repr());
  };

  return (
    <div style={{ width: "100vw", height: "100vh", display: "flex", flexDirection: "column" }}>
      <div style={{ padding: "10px 20px", borderBottom: "1px solid #ccc", background: "#f8f9fa" }}>
        <h2 style={{ margin: "0 0 15px 0", fontSize: "18px" }}>Canvas - Draw with Mouse</h2>

        <div style={{ display: "flex", gap: "20px", flexWrap: "wrap", alignItems: "center" }}>
          {/* size slider */}
          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <label style={{ fontSize: "14px", minWidth: "40px" }}>Size:</label>
            <input
              type="range"
              min="1"
              max="50"
              value={size.value}
              onChange={(e) => (size.value = Number(e.target.value))}
              style={{ width: "100px" }}
            />
            <span style={{ fontSize: "12px", minWidth: "25px" }}>{size}</span>
          </div>

          {/* thinning slider */}
          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <label style={{ fontSize: "14px", minWidth: "60px" }}>Thinning:</label>
            <input
              type="range"
              min="-1"
              max="1"
              step="0.05"
              value={thinning.value}
              onChange={(e) => (thinning.value = Number(e.target.value))}
              style={{ width: "100px" }}
            />
            <span style={{ fontSize: "12px", minWidth: "25px" }}>{thinning}</span>
          </div>

          {/* smoothing slider */}
          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <label style={{ fontSize: "14px", minWidth: "70px" }}>Smoothing:</label>
            <input
              type="range"
              min="0"
              max="1"
              step="0.1"
              value={smoothing.value}
              onChange={(e) => (smoothing.value = Number(e.target.value))}
              style={{ width: "100px" }}
            />
            <span style={{ fontSize: "12px", minWidth: "25px" }}>{smoothing}</span>
          </div>

          {/* streamline slider */}
          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <label style={{ fontSize: "14px", minWidth: "70px" }}>Streamline:</label>
            <input
              type="range"
              min="0"
              max="1"
              step="0.1"
              value={streamline.value}
              onChange={(e) => (streamline.value = Number(e.target.value))}
              style={{ width: "100px" }}
            />
            <span style={{ fontSize: "12px", minWidth: "25px" }}>{streamline}</span>
          </div>
        </div>

        <div style={{ display: "flex", gap: "20px", marginTop: "10px", alignItems: "center" }}>
          {/* simulate pressure toggle */}
          <label style={{ display: "flex", alignItems: "center", gap: "8px", fontSize: "14px" }}>
            <input
              type="checkbox"
              checked={simulatePressure.value}
              onChange={(e) => (simulatePressure.value = e.target.checked)}
            />
            Simulate Pressure
          </label>

          {/* taper start toggle */}
          <label style={{ display: "flex", alignItems: "center", gap: "8px", fontSize: "14px" }}>
            <input
              type="checkbox"
              checked={taperStart.value}
              onChange={(e) => (taperStart.value = e.target.checked)}
            />
            Taper Start
          </label>

          {/* taper end toggle */}
          <label style={{ display: "flex", alignItems: "center", gap: "8px", fontSize: "14px" }}>
            <input
              type="checkbox"
              checked={taperEnd.value}
              onChange={(e) => (taperEnd.value = e.target.checked)}
            />
            Taper End
          </label>

          {/* clear button */}
          <button
            onClick={() => {
              lines.value = [];
            }}
            style={{
              padding: "6px 12px",
              fontSize: "14px",
              border: "1px solid #ccc",
              borderRadius: "4px",
              background: "#fff",
              cursor: "pointer",
            }}
          >
            Clear
          </button>
        </div>
      </div>

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
          <path
            key={index}
            d={renderStroke(line.points, strokeOptions.value, { isComplete: true })}
            fill="#2563eb"
            stroke="none"
          />
        ))}

        {/* render current line being drawn */}
        {currentLine.value && currentLine.value.points.length > 1 && (
          <path
            d={renderStroke(currentLine.value.points, strokeOptions.value, { isComplete: false })}
            fill="#94a3b8"
            stroke="none"
          />
        )}
      </svg>
    </div>
  );
};

export default Canvas;

import { renderStroke } from "@destack-web/shared/freehand/svg";
import { computed, Signal, signal } from "@preact/signals-react";
import { Easing, Line, Stroke, StrokeCap, StrokeType, Vector2 } from "destack";
import React, { useRef } from "react";

const size = signal(12);
const thinning = signal(0.5);
const smoothing = signal(0.62);
const streamline = signal(0.25);
const simulatePressure = signal(true);
const taperStart = signal(false);
const taperEnd = signal(false);
const showPoints = signal(false);
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
    const point = getMousePosition(event);
    isDrawing.value = true;
    lastMousePosition.value = point;
    currentLine.value = new Line({ points: [point] });
  };

  // add points on mouse move
  const handleMouseMove = (event: React.MouseEvent<SVGSVGElement>) => {
    const currentPoint = getMousePosition(event);

    if (isDrawing.value && currentLine.value) {
      const lastPoint = currentLine.value.points[currentLine.value.points.length - 1];
      if (lastPoint.x !== currentPoint.x || lastPoint.y !== currentPoint.y) {
        currentLine.value = new Line({
          points: [...currentLine.value.points, currentPoint],
        });
      }
    }
  };

  // finish drawing on mouse up
  const handleMouseUp = () => {
    if (isDrawing.value && currentLine.value) {
      isDrawing.value = false;
      lines.value = [...lines.value, currentLine.value];
      currentLine.value = null;
      lastMousePosition.value = null;
    }
  };

  return (
    <div style={{ width: "100vw", height: "100vh", display: "flex", flexDirection: "column" }}>
      <div style={{ padding: "15px 20px", borderBottom: "1px solid #ccc", background: "#f8f9fa" }}>
        <h2 style={{ margin: "0 0 15px 0", fontSize: "18px" }}>Canvas Editor</h2>

        {/* stroke settings */}
        <div
          style={{
            display: "flex",
            gap: "15px",
            flexWrap: "wrap",
            alignItems: "center",
            marginBottom: "10px",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <label style={{ fontSize: "14px", minWidth: "40px" }}>Size:</label>
            <input
              type="range"
              min="1"
              max="50"
              value={size.value}
              onChange={(e) => (size.value = Number(e.target.value))}
              style={{ width: "80px" }}
            />
            <span style={{ fontSize: "12px", minWidth: "25px" }}>{size}</span>
          </div>

          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <label style={{ fontSize: "14px", minWidth: "60px" }}>Thinning:</label>
            <input
              type="range"
              min="-1"
              max="1"
              step="0.05"
              value={thinning.value}
              onChange={(e) => (thinning.value = Number(e.target.value))}
              style={{ width: "80px" }}
            />
            <span style={{ fontSize: "12px", minWidth: "35px" }}>{thinning.value.toFixed(2)}</span>
          </div>

          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <label style={{ fontSize: "14px", minWidth: "70px" }}>Smoothing:</label>
            <input
              type="range"
              min="0"
              max="1"
              step="0.1"
              value={smoothing.value}
              onChange={(e) => (smoothing.value = Number(e.target.value))}
              style={{ width: "80px" }}
            />
            <span style={{ fontSize: "12px", minWidth: "25px" }}>{smoothing}</span>
          </div>

          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <label style={{ fontSize: "14px", minWidth: "70px" }}>Streamline:</label>
            <input
              type="range"
              min="0"
              max="1"
              step="0.1"
              value={streamline.value}
              onChange={(e) => (streamline.value = Number(e.target.value))}
              style={{ width: "80px" }}
            />
            <span style={{ fontSize: "12px", minWidth: "25px" }}>{streamline}</span>
          </div>

          <label style={{ display: "flex", alignItems: "center", gap: "6px", fontSize: "14px" }}>
            <input
              type="checkbox"
              checked={showPoints.value}
              onChange={(e) => (showPoints.value = e.target.checked)}
            />
            Show Points
          </label>
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
          <g key={index}>
            <path
              d={renderStroke(line.points, strokeOptions.value, { isComplete: true })}
              fill="#2563eb"
              stroke="none"
            />
            {/* render individual points if enabled */}
            {showPoints.value &&
              line.points.map((point, pointIndex) => (
                <circle
                  key={pointIndex}
                  cx={point.x}
                  cy={point.y}
                  r="3"
                  fill="#dc2626"
                  stroke="black"
                  strokeWidth="1"
                />
              ))}
          </g>
        ))}

        {/* render current line being drawn */}
        {currentLine.value && currentLine.value.points.length > 1 && (
          <g>
            <path
              d={renderStroke(currentLine.value.points, strokeOptions.value, { isComplete: false })}
              fill="#94a3b8"
              stroke="none"
            />
            {/* render individual points for current line if enabled */}
            {showPoints.value &&
              currentLine.value.points.map((point, pointIndex) => (
                <circle
                  key={pointIndex}
                  cx={point.x}
                  cy={point.y}
                  r="2"
                  fill="#dc2626"
                  stroke="black"
                  strokeWidth="1"
                />
              ))}
          </g>
        )}
      </svg>
    </div>
  );
};

export default Canvas;

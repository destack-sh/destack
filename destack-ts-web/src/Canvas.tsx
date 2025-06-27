import { EASINGS } from "@destack-web/shared/easings";
import { renderStroke } from "@destack-web/shared/freehand/svg";
import { StrokeOptions } from "@destack-web/shared/freehand/types";
import { computed, Signal, signal, useSignal } from "@preact/signals-react";
import { Vector3 } from "destack";
import React, { useEffect, useRef } from "react";

type Line = {
  points: Vector3[];
};

const currentLine = signal<Line | null>(null);
const lines = signal<Line[]>([]);

export const Canvas: React.FC = () => {
  const size = useSignal(12);
  const thinning = useSignal(0.5);
  const smoothing = useSignal(0.4);
  const streamline = useSignal(0.2);
  const simulatePressure = useSignal(true);
  const taperStart = useSignal(false);
  const taperEnd = useSignal(false);
  const isDrawing = useSignal(false);
  const lastMousePosition = useSignal<Vector3 | null>(null);

  const svgRef = useRef<SVGSVGElement>(null);
  const animationFrameId = useRef<number | null>(null);

  const strokeOptions: Signal<StrokeOptions> = computed(() => ({
    size: size.value,
    thinning: thinning.value,
    smoothing: smoothing.value,
    streamline: streamline.value,
    simulatePressure: simulatePressure.value,
    start: {
      cap: true,
      taper: taperStart.value,
      easing: EASINGS.easeOutCubic,
    },
    end: {
      cap: true,
      taper: taperEnd.value,
      easing: EASINGS.easeOutCubic,
    },
  }));

  const getMousePosition = (event: React.MouseEvent<SVGSVGElement>): Vector3 => {
    if (!svgRef.current) {
      throw new Error("SVG element not found");
    }

    const rect = svgRef.current.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    const pressure = 0.5;

    return new Vector3({ x, y, z: pressure });
  };

  // high-frequency sampling using requestAnimationFrame
  console.log("init");
  const sampleCurrentPosition = () => {
    if (!isDrawing.value || !lastMousePosition.value || currentLine.value == null) return;

    // add the current mouse position if we have one
    const lastPoint = currentLine.value.points[currentLine.value.points.length - 1];
    const currentPoint = lastMousePosition.value;

    // only add if position has changed (avoid duplicate points)
    if (lastPoint == null || lastPoint.x != currentPoint.x || lastPoint.y != currentPoint.y) {
      console.log("add point", currentLine.value.points.length, {
        x: currentPoint.x,
        y: currentPoint.y,
      });
      currentLine.value = {
        points: [...currentLine.value.points, currentPoint],
      };
    }

    // continue sampling
    animationFrameId.current = requestAnimationFrame(sampleCurrentPosition);
  };

  const handleMouseDown = (event: React.MouseEvent<SVGSVGElement>) => {
    isDrawing.value = true;
    const point = getMousePosition(event);
    lastMousePosition.value = point;
    currentLine.value = { points: [point] };

    // start high-frequency sampling
    animationFrameId.current = requestAnimationFrame(sampleCurrentPosition);
  };

  const handleMouseMove = (event: React.MouseEvent<SVGSVGElement>) => {
    if (!isDrawing.value) return;

    // just update the last mouse position, let RAF handle the sampling
    lastMousePosition.value = getMousePosition(event);
  };

  const handleMouseUp = () => {
    if (!isDrawing.value || !currentLine.value) return;

    isDrawing.value = false;
    lines.value = [...lines.value, currentLine.value];
    currentLine.value = null;
    lastMousePosition.value = null;

    // cleanup timers
    if (animationFrameId.current) {
      cancelAnimationFrame(animationFrameId.current);
      animationFrameId.current = null;
    }
  };

  // cleanup on unmount
  useEffect(() => {
    return () => {
      if (animationFrameId.current) {
        cancelAnimationFrame(animationFrameId.current);
      }
    };
  }, []);

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
              min="0"
              max="1"
              step="0.1"
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
            d={renderStroke(line.points, { ...strokeOptions.value, last: true })}
            fill="#2563eb"
            stroke="none"
          />
        ))}

        {/* render current line being drawn */}
        {currentLine.value && currentLine.value.points.length > 1 && (
          <path
            d={renderStroke(currentLine.value.points, { ...strokeOptions.value })}
            fill="#94a3b8"
            stroke="red"
          />
        )}
      </svg>
    </div>
  );
};

export default Canvas;

import { EASINGS } from "@destack-web/shared/easings";
import { renderStroke } from "@destack-web/shared/freehand/svg";
import { StrokeOptions } from "@destack-web/shared/freehand/types";
import { signal } from "@preact/signals-react";
import { Vector3 } from "destack";
import React, { useCallback, useEffect, useRef, useState } from "react";

type Line = {
  points: Vector3[];
};

const currentLine = signal<Line | null>(null);
const lines = signal<Line[]>([]);

export const Canvas: React.FC = () => {
  const svgRef = useRef<SVGSVGElement>(null);
  const isDrawing = useRef(false);
  const lastMousePosition = useRef<Vector3 | null>(null);
  const animationFrameId = useRef<number | null>(null);

  // stroke option controls
  const [size, setSize] = useState(12);
  const [thinning, setThinning] = useState(0.5);
  const [smoothing, setSmoothing] = useState(0.4);
  const [streamline, setStreamline] = useState(0.2);
  const [simulatePressure, setSimulatePressure] = useState(true);
  const [taperStart, setTaperStart] = useState(false);
  const [taperEnd, setTaperEnd] = useState(false);

  const strokeOptions: StrokeOptions = {
    size,
    thinning,
    smoothing,
    streamline,
    simulatePressure,
    start: {
      cap: true,
      taper: taperStart,
      easing: EASINGS.easeOutCubic,
    },
    end: {
      cap: true,
      taper: taperEnd,
      easing: EASINGS.easeOutCubic,
    },
  };

  const getMousePosition = (event: React.MouseEvent<SVGSVGElement>): Vector3 => {
    if (!svgRef.current) return new Vector3({ x: 0, y: 0, z: 0.5 });

    const rect = svgRef.current.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    const pressure = 0.5;

    return new Vector3({ x, y, z: pressure });
  };

  // high-frequency sampling using requestAnimationFrame
  const sampleCurrentPosition = useCallback(() => {
    if (!isDrawing.current || !lastMousePosition.current) return;

    // add the current mouse position if we have one
    if (currentLine.value) {
      const lastPoint = currentLine.value.points[currentLine.value.points.length - 1];
      const currentPoint = lastMousePosition.current;

      // only add if position has changed significantly (avoid duplicate points)
      if (lastPoint == null || lastPoint.x != currentPoint.x || lastPoint.y != currentPoint.y) {
        console.log("add point", currentLine.value.points.length, currentPoint.x, currentPoint.y);
        currentLine.value = {
          points: [...currentLine.value.points, currentPoint],
        };
      }
    }

    // continue sampling
    if (isDrawing.current) {
      animationFrameId.current = requestAnimationFrame(sampleCurrentPosition);
    }
  }, []);

  const handleMouseDown = (event: React.MouseEvent<SVGSVGElement>) => {
    isDrawing.current = true;
    const point = getMousePosition(event);
    lastMousePosition.current = point;
    currentLine.value = { points: [point] };

    // start high-frequency sampling
    animationFrameId.current = requestAnimationFrame(sampleCurrentPosition);
  };

  const handleMouseMove = (event: React.MouseEvent<SVGSVGElement>) => {
    if (!isDrawing.current) return;

    // just update the last mouse position, let RAF handle the sampling
    lastMousePosition.current = getMousePosition(event);
  };

  const handleMouseUp = () => {
    if (!isDrawing.current || !currentLine.value) return;

    isDrawing.current = false;
    lines.value = [...lines.value, currentLine.value];
    currentLine.value = null;
    lastMousePosition.current = null;

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
              value={size}
              onChange={(e) => setSize(Number(e.target.value))}
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
              value={thinning}
              onChange={(e) => setThinning(Number(e.target.value))}
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
              value={smoothing}
              onChange={(e) => setSmoothing(Number(e.target.value))}
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
              value={streamline}
              onChange={(e) => setStreamline(Number(e.target.value))}
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
              checked={simulatePressure}
              onChange={(e) => setSimulatePressure(e.target.checked)}
            />
            Simulate Pressure
          </label>

          {/* taper start toggle */}
          <label style={{ display: "flex", alignItems: "center", gap: "8px", fontSize: "14px" }}>
            <input
              type="checkbox"
              checked={taperStart}
              onChange={(e) => setTaperStart(e.target.checked)}
            />
            Taper Start
          </label>

          {/* taper end toggle */}
          <label style={{ display: "flex", alignItems: "center", gap: "8px", fontSize: "14px" }}>
            <input
              type="checkbox"
              checked={taperEnd}
              onChange={(e) => setTaperEnd(e.target.checked)}
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
            d={renderStroke(line.points, { ...strokeOptions, last: true })}
            fill="#2563eb"
            stroke="none"
          />
        ))}

        {/* render current line being drawn */}
        {currentLine.value && currentLine.value.points.length > 1 && (
          <path
            d={renderStroke(currentLine.value.points, strokeOptions)}
            fill="#94a3b8"
            stroke="red"
          />
        )}
      </svg>
    </div>
  );
};

export default Canvas;

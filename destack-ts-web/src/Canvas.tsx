import { renderStroke } from "@destack-web/shared/freehand/svg";
import { computed, Signal, signal } from "@preact/signals-react";
import { Easing, Line, Stroke, StrokeCap, StrokeType, Vector2 } from "destack";
import React, { useEffect, useRef } from "react";

// tool types
type Tool = "paint" | "select";

// resize handle types
type ResizeHandle = "nw" | "ne" | "sw" | "se";

const currentTool = signal<Tool>("paint");
const size = signal(12);
const thinning = signal(0.5);
const smoothing = signal(0.62);
const streamline = signal(0.62);
const simulatePressure = signal(true);
const taperStart = signal(false);
const taperEnd = signal(false);
const showPoints = signal(false);
const isDrawing = signal(false);
const isMoving = signal(false);
const isResizing = signal(false);
const activeResizeHandle = signal<ResizeHandle | null>(null);
const lastMousePosition = signal<Vector2 | null>(null);
const moveStartPosition = signal<Vector2 | null>(null);
const resizeStartPosition = signal<Vector2 | null>(null);
const resizeStartBounds = signal<{ x: number; y: number; width: number; height: number } | null>(
  null,
);
const originalLineBeforeResize = signal<Line | null>(null);
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

  // keyboard event handler for escape key
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        selectedLine.value = null;
        isMoving.value = false;
        isResizing.value = false;
        activeResizeHandle.value = null;
        moveStartPosition.value = null;
        resizeStartPosition.value = null;
        resizeStartBounds.value = null;
        originalLineBeforeResize.value = null;
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, []);

  const getMousePosition = (event: React.MouseEvent<SVGSVGElement>): Vector2 => {
    if (!svgRef.current) {
      throw new Error("SVG element not found");
    }
    const rect = svgRef.current.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    return new Vector2({ x, y });
  };

  // check if a point is close to any point in a line (for hit detection)
  const isPointNearLine = (point: Vector2, line: Line, threshold: number = 15): boolean => {
    return line.points.some((linePoint) => {
      const dx = point.x - linePoint.x;
      const dy = point.y - linePoint.y;
      return Math.sqrt(dx * dx + dy * dy) < threshold;
    });
  };

  // find which line was clicked (if any)
  const getLineAtPoint = (point: Vector2): Line | null => {
    // check lines in reverse order (most recently drawn first)
    for (let i = lines.value.length - 1; i >= 0; i--) {
      if (isPointNearLine(point, lines.value[i])) {
        return lines.value[i];
      }
    }
    return null;
  };

  // calculate bounding box of a line
  const getLineBounds = (line: Line): { x: number; y: number; width: number; height: number } => {
    if (line.points.length === 0) return { x: 0, y: 0, width: 0, height: 0 };

    let minX = line.points[0].x;
    let maxX = line.points[0].x;
    let minY = line.points[0].y;
    let maxY = line.points[0].y;

    line.points.forEach((point) => {
      minX = Math.min(minX, point.x);
      maxX = Math.max(maxX, point.x);
      minY = Math.min(minY, point.y);
      maxY = Math.max(maxY, point.y);
    });

    const padding = 10;
    return {
      x: minX - padding,
      y: minY - padding,
      width: maxX - minX + 2 * padding,
      height: maxY - minY + 2 * padding,
    };
  };

  // get resize handle positions
  const getResizeHandles = (bounds: { x: number; y: number; width: number; height: number }) => {
    const handleSize = 8;
    return {
      nw: { x: bounds.x - handleSize / 2, y: bounds.y - handleSize / 2 },
      ne: { x: bounds.x + bounds.width - handleSize / 2, y: bounds.y - handleSize / 2 },
      sw: { x: bounds.x - handleSize / 2, y: bounds.y + bounds.height - handleSize / 2 },
      se: {
        x: bounds.x + bounds.width - handleSize / 2,
        y: bounds.y + bounds.height - handleSize / 2,
      },
    };
  };

  // check if point is near a resize handle
  const getResizeHandleAtPoint = (
    point: Vector2,
    bounds: { x: number; y: number; width: number; height: number },
  ): ResizeHandle | null => {
    const handles = getResizeHandles(bounds);
    const threshold = 12;

    for (const [handle, pos] of Object.entries(handles)) {
      const dx = point.x - (pos.x + 4); // +4 for handle center
      const dy = point.y - (pos.y + 4);
      if (Math.sqrt(dx * dx + dy * dy) < threshold) {
        return handle as ResizeHandle;
      }
    }
    return null;
  };

  // apply scaling to a line
  const scaleLine = (
    line: Line,
    scaleX: number,
    scaleY: number,
    centerX: number,
    centerY: number,
  ): Line => {
    const scaledPoints = line.points.map((point) => {
      const relativeX = point.x - centerX;
      const relativeY = point.y - centerY;
      return new Vector2({
        x: centerX + relativeX * scaleX,
        y: centerY + relativeY * scaleY,
      });
    });
    return new Line({ points: scaledPoints });
  };

  // begin drawing on mouse down
  const handleMouseDown = (event: React.MouseEvent<SVGSVGElement>) => {
    const point = getMousePosition(event);

    if (currentTool.value === "paint") {
      // paint mode - always start drawing
      selectedLine.value = null;
      isDrawing.value = true;
      lastMousePosition.value = point;
      currentLine.value = new Line({ points: [point] });
    } else if (currentTool.value === "select") {
      // select mode - handle selection, movement, and resizing
      if (selectedLine.value) {
        const bounds = getLineBounds(selectedLine.value);
        const resizeHandle = getResizeHandleAtPoint(point, bounds);

        if (resizeHandle) {
          // start resizing
          isResizing.value = true;
          activeResizeHandle.value = resizeHandle;
          resizeStartPosition.value = point;
          resizeStartBounds.value = bounds;
          originalLineBeforeResize.value = selectedLine.value;
        } else {
          const clickedLine = getLineAtPoint(point);
          if (clickedLine === selectedLine.value) {
            // start moving the selected line
            isMoving.value = true;
            moveStartPosition.value = point;
          } else if (clickedLine) {
            // select a different line
            selectedLine.value = clickedLine;
          } else {
            // clicked on empty space - deselect
            selectedLine.value = null;
          }
        }
      } else {
        // no line selected - try to select one
        const clickedLine = getLineAtPoint(point);
        if (clickedLine) {
          selectedLine.value = clickedLine;
        }
      }
    }
  };

  // add points on mouse move
  const handleMouseMove = (event: React.MouseEvent<SVGSVGElement>) => {
    const currentPoint = getMousePosition(event);

    if (
      isResizing.value &&
      selectedLine.value &&
      resizeStartPosition.value &&
      resizeStartBounds.value &&
      activeResizeHandle.value &&
      originalLineBeforeResize.value
    ) {
      // resize the selected line
      const bounds = resizeStartBounds.value;

      // calculate new bounds based on current mouse position and active handle
      let newBounds = { ...bounds };

      switch (activeResizeHandle.value) {
        case "nw":
          newBounds.width = bounds.x + bounds.width - currentPoint.x;
          newBounds.height = bounds.y + bounds.height - currentPoint.y;
          newBounds.x = currentPoint.x;
          newBounds.y = currentPoint.y;
          break;
        case "ne":
          newBounds.width = currentPoint.x - bounds.x;
          newBounds.height = bounds.y + bounds.height - currentPoint.y;
          newBounds.y = currentPoint.y;
          break;
        case "sw":
          newBounds.width = bounds.x + bounds.width - currentPoint.x;
          newBounds.height = currentPoint.y - bounds.y;
          newBounds.x = currentPoint.x;
          break;
        case "se":
          newBounds.width = currentPoint.x - bounds.x;
          newBounds.height = currentPoint.y - bounds.y;
          break;
      }

      // ensure minimum size
      const minSize = 20;
      if (Math.abs(newBounds.width) < minSize || Math.abs(newBounds.height) < minSize) {
        return;
      }

      // calculate scale factors
      const scaleX = newBounds.width / bounds.width;
      const scaleY = newBounds.height / bounds.height;

      // determine center point for scaling (opposite corner)
      let centerX: number;
      let centerY: number;

      switch (activeResizeHandle.value) {
        case "nw":
          centerX = bounds.x + bounds.width;
          centerY = bounds.y + bounds.height;
          break;
        case "ne":
          centerX = bounds.x;
          centerY = bounds.y + bounds.height;
          break;
        case "sw":
          centerX = bounds.x + bounds.width;
          centerY = bounds.y;
          break;
        case "se":
          centerX = bounds.x;
          centerY = bounds.y;
          break;
        default:
          return;
      }

      // find the index of the original line and update it
      const lineIndex = lines.value.findIndex(
        (line) => line === originalLineBeforeResize.value || line === selectedLine.value,
      );
      if (lineIndex !== -1) {
        const scaledLine = scaleLine(
          originalLineBeforeResize.value,
          scaleX,
          scaleY,
          centerX,
          centerY,
        );

        // update lines array with the scaled line
        const newLines = [...lines.value];
        newLines[lineIndex] = scaledLine;
        lines.value = newLines;
        selectedLine.value = scaledLine;
      }
    } else if (isMoving.value && selectedLine.value && moveStartPosition.value) {
      // move the selected line
      const dx = currentPoint.x - moveStartPosition.value.x;
      const dy = currentPoint.y - moveStartPosition.value.y;

      // find the index of the selected line and update it
      const lineIndex = lines.value.findIndex((line) => line === selectedLine.value);
      if (lineIndex !== -1) {
        const originalLine = lines.value[lineIndex];
        const movedLine = new Line({
          points: originalLine.points.map(
            (point) => new Vector2({ x: point.x + dx, y: point.y + dy }),
          ),
        });

        // update lines array with the moved line
        const newLines = [...lines.value];
        newLines[lineIndex] = movedLine;
        lines.value = newLines;
        selectedLine.value = movedLine;
      }

      moveStartPosition.value = currentPoint;
    } else if (isDrawing.value && currentLine.value) {
      // regular drawing mode
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
    if (isResizing.value) {
      // finish resizing
      isResizing.value = false;
      activeResizeHandle.value = null;
      resizeStartPosition.value = null;
      resizeStartBounds.value = null;
      originalLineBeforeResize.value = null;
    } else if (isMoving.value) {
      // finish moving
      isMoving.value = false;
      moveStartPosition.value = null;
    } else if (isDrawing.value && currentLine.value) {
      // finish drawing
      isDrawing.value = false;
      lines.value = [...lines.value, currentLine.value];
      currentLine.value = null;
      lastMousePosition.value = null;
    }
  };

  // get cursor based on current state
  const getCursor = (): string => {
    if (isResizing.value) {
      switch (activeResizeHandle.value) {
        case "nw":
        case "se":
          return "nw-resize";
        case "ne":
        case "sw":
          return "ne-resize";
        default:
          return "grabbing";
      }
    }
    if (isMoving.value) return "grabbing";
    if (currentTool.value === "select") {
      if (selectedLine.value) {
        const bounds = getLineBounds(selectedLine.value);
        // This would need mouse position to check handles, simplified for now
        return "grab";
      }
      return "default";
    }
    return "crosshair";
  };

  return (
    <div style={{ width: "100vw", height: "100vh", display: "flex", flexDirection: "column" }}>
      <div style={{ padding: "15px 20px", borderBottom: "1px solid #ccc", background: "#f8f9fa" }}>
        <h2 style={{ margin: "0 0 15px 0", fontSize: "18px" }}>Canvas Editor</h2>

        {/* tool selection */}
        <div style={{ display: "flex", gap: "10px", marginBottom: "15px", alignItems: "center" }}>
          <span style={{ fontSize: "14px", fontWeight: "500" }}>Tools:</span>
          <button
            onClick={() => {
              currentTool.value = "paint";
              selectedLine.value = null;
            }}
            style={{
              padding: "8px 16px",
              fontSize: "14px",
              border: "1px solid #ccc",
              borderRadius: "4px",
              background: currentTool.value === "paint" ? "#2563eb" : "#fff",
              color: currentTool.value === "paint" ? "white" : "black",
              cursor: "pointer",
            }}
          >
            🖌️ Paint
          </button>
          <button
            onClick={() => (currentTool.value = "select")}
            style={{
              padding: "8px 16px",
              fontSize: "14px",
              border: "1px solid #ccc",
              borderRadius: "4px",
              background: currentTool.value === "select" ? "#2563eb" : "#fff",
              color: currentTool.value === "select" ? "white" : "black",
              cursor: "pointer",
            }}
          >
            👆 Select
          </button>
        </div>

        {/* stroke settings - only show when paint tool is active */}
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
          cursor: getCursor(),
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

        {/* render selection highlight and resize handles */}
        {selectedLine.value &&
          (() => {
            const bounds = getLineBounds(selectedLine.value);
            const highlightColor = "#ff6b35";
            const strokeWidth = 2;
            const handles = getResizeHandles(bounds);

            return (
              <g>
                {/* selection bounding box */}
                <rect
                  x={bounds.x}
                  y={bounds.y}
                  width={bounds.width}
                  height={bounds.height}
                  fill="none"
                  stroke={highlightColor}
                  strokeWidth={strokeWidth}
                  strokeDasharray="4,4"
                  rx="4"
                />

                {/* smooth trajectory line following the same path as the stroke */}
                {selectedLine.value.points.length > 1 && (
                  <path
                    d={renderStroke(selectedLine.value.points, strokeOptions.value, {
                      isComplete: true,
                    })}
                    fill="none"
                    stroke={highlightColor}
                    strokeWidth={strokeWidth}
                    opacity="0.8"
                  />
                )}

                {/* resize handles */}
                {Object.entries(handles).map(([handle, pos]) => (
                  <rect
                    key={handle}
                    x={pos.x}
                    y={pos.y}
                    width="8"
                    height="8"
                    fill="white"
                    stroke={highlightColor}
                    strokeWidth="2"
                    rx="1"
                    style={{
                      cursor: handle === "nw" || handle === "se" ? "nw-resize" : "ne-resize",
                    }}
                  />
                ))}
              </g>
            );
          })()}
      </svg>
    </div>
  );
};

export default Canvas;

import { averageVector2String, toDomPrecision, vector2String } from "@destack-web/shared/dom/utils";
import { getStrokeOutlineTracks, getStrokePoints } from "@destack-web/shared/freehand/stroke";
import { Stroke, StrokePoint, Vector3 } from "destack";

/**
 * Generate SVG path data for stroke with ink-like rendering.
 * Uses partitioning at elbows for more natural line appearance.
 */
export function renderStroke(
  points: readonly Vector3[],
  stroke: Stroke,
  options: { isComplete: boolean },
) {
  const strokePoints = getStrokePoints(points, stroke, options);
  const partitions = partitionStroke(strokePoints);

  const svgPartitions = [];
  for (const partition of partitions) {
    svgPartitions.push(renderPartition(partition, stroke));
  }
  return svgPartitions.join("");
}

/**
 * Generate SVG circle path for single point strokes.
 */
function renderCirclePath(cx: number, cy: number, r: number) {
  return (
    "M " +
    cx +
    " " +
    cy +
    " m -" +
    r +
    ", 0 a " +
    r +
    "," +
    r +
    " 0 1,1 " +
    r * 2 +
    ",0 a " +
    r +
    "," +
    r +
    " 0 1,1 -" +
    r * 2 +
    ",0"
  );
}

/**
 * Render a partition of stroke points as SVG path.
 * Handles single points as circles and multi-point strokes as paths with caps.
 */
function renderPartition(strokePoints: StrokePoint[], options: Stroke): string {
  if (strokePoints.length === 0) return "";

  if (strokePoints.length === 1) {
    return renderCirclePath(
      strokePoints[0].point.x,
      strokePoints[0].point.y,
      strokePoints[0].radius,
    );
  }

  const { left, right } = getStrokeOutlineTracks(strokePoints, options);
  right.reverse();
  let svg = `M${vector2String(left[0])}T`;

  // draw left track
  for (let i = 1; i < left.length; i++) {
    svg += averageVector2String(left[i - 1], left[i]);
  }

  // draw end cap arc
  {
    const point = strokePoints[strokePoints.length - 1];
    const radius = point.radius;
    const direction = point.direction.per().mul(-1);
    const arcStart = point.point.add(direction.mul(radius));
    const arcEnd = point.point.add(direction.mul(-radius));
    svg += `${vector2String(arcStart)}A${toDomPrecision(radius)},${toDomPrecision(
      radius,
    )} 0 0 1 ${vector2String(arcEnd)}T`;
  }

  // draw right track
  for (let i = 1; i < right.length; i++) {
    svg += averageVector2String(right[i - 1], right[i]);
  }

  // draw start cap arc
  {
    const point = strokePoints[0];
    const radius = point.radius;
    const direction = point.direction.per();
    const arcStart = point.point.add(direction.mul(radius));
    const arcEnd = point.point.add(direction.mul(-radius));
    svg += `${vector2String(arcStart)}A${toDomPrecision(radius)},${toDomPrecision(
      radius,
    )} 0 0 1 ${vector2String(arcEnd)}Z`;
  }

  return svg;
}

/**
 * Turn an array of stroke points into a path of quadratic curves.
 * Creates smooth curves between stroke points for SVG rendering.
 */
export function renderStrokePath(points: StrokePoint[], closed = false): string | null {
  const len = points.length;
  if (len < 2) {
    return null; // nothing to render
  }

  let a = points[0].point;
  let b = points[1].point;
  if (len === 2) {
    return `M${vector2String(a)}L${vector2String(b)}`;
  }

  let result = "";
  for (let i = 2, max = len - 1; i < max; i++) {
    a = points[i].point;
    b = points[i + 1].point;
    result += averageVector2String(a, b);
  }

  if (closed) {
    // if closed, draw a curve from the last point to the first
    return `M${averageVector2String(points[0].point, points[1].point)}Q${vector2String(points[1].point)}${averageVector2String(
      points[1].point,
      points[2].point,
    )}T${result}${averageVector2String(points[len - 1].point, points[0].point)}${averageVector2String(
      points[0].point,
      points[1].point,
    )}Z`;
  } else {
    // if not closed, draw a curve starting at the first point and
    // ending at the midpoint of the last and second-last point, then
    // complete the curve with a line segment to the last point
    return `M${vector2String(points[0].point)}Q${vector2String(points[1].point)}${averageVector2String(
      points[1].point,
      points[2].point,
    )}${points.length > 3 ? "T" : ""}${result}L${vector2String(points[len - 1].point)}`;
  }
}

/**
 * Partition stroke points at sharp "elbow" angles.
 * Creates separate segments for better rendering of complex paths.
 */
function partitionStroke(points: StrokePoint[]): StrokePoint[][] {
  if (points.length <= 2) return [points];

  const partitions: StrokePoint[][] = [];
  let currentPartition: StrokePoint[] = [points[0]];
  let prevV = points[1].point.sub(points[0].point).normalize();
  let nextV: Vector3;
  let dpr: number;
  let prevPoint: StrokePoint, thisPoint: StrokePoint, nextPoint: StrokePoint;

  for (let i = 1, n = points.length; i < n - 1; i++) {
    prevPoint = points[i - 1];
    thisPoint = points[i];
    nextPoint = points[i + 1];

    nextV = nextPoint.point.sub(thisPoint.point).normalize();
    dpr = prevV.dot(nextV);
    prevV = nextV;

    if (dpr < -0.8) {
      // always treat such acute angles as elbows
      // and use the extended .input point as the elbow point for swooshiness in fast zaggy lines
      const elbowPoint = new StrokePoint({
        ...thisPoint,
        point: thisPoint.originalPoint,
      });
      currentPartition.push(elbowPoint);
      partitions.push(cleanUpPartition(currentPartition));
      currentPartition = [elbowPoint];
      continue;
    }

    currentPartition.push(thisPoint);

    if (dpr > 0.7) {
      // not an elbow
      continue;
    }

    // we have a reasonably acute angle but it might not be an elbow if it's far
    if (
      (prevPoint.point.distance2(thisPoint.point) + thisPoint.point.distance2(nextPoint.point)) /
        ((prevPoint.radius + thisPoint.radius + nextPoint.radius) / 3) ** 2 <
      1.5
    ) {
      // point is also close to its neighbors, probably a hard elbow
      currentPartition.push(thisPoint);
      partitions.push(cleanUpPartition(currentPartition));
      currentPartition = [thisPoint];
      continue;
    }
  }

  currentPartition.push(points[points.length - 1]);
  partitions.push(cleanUpPartition(currentPartition));

  return partitions;
}

/**
 * Clean up a partition by removing points too close to start/end.
 * Adjust cap point vectors to point to nearest neighbors.
 */
function cleanUpPartition(partition: StrokePoint[]) {
  // clean up start of partition (remove points that are too close to the start)
  const startPoint = partition[0];
  let nextPoint: StrokePoint;

  while (partition.length > 2) {
    nextPoint = partition[1];
    if (
      startPoint.point.distance2(nextPoint.point) <
      (((startPoint.radius + nextPoint.radius) / 2) * 0.5) ** 2
    ) {
      partition.splice(1, 1);
    } else {
      break;
    }
  }

  // clean up end of partition in the same fashion
  const endPoint = partition[partition.length - 1];
  let prevPoint: StrokePoint;

  while (partition.length > 2) {
    prevPoint = partition[partition.length - 2];
    if (
      endPoint.point.distance2(prevPoint.point) <
      (((endPoint.radius + prevPoint.radius) / 2) * 0.5) ** 2
    ) {
      partition.splice(partition.length - 2, 1);
    } else {
      break;
    }
  }

  // now readjust the cap point vectors to point to their nearest neighbors
  if (partition.length > 1) {
    partition[0] = new StrokePoint({
      ...partition[0],
      direction: partition[0].point.sub(partition[1].point).normalize(),
    });
    partition[partition.length - 1] = new StrokePoint({
      ...partition[partition.length - 1],
      direction: partition[partition.length - 2].point
        .sub(partition[partition.length - 1].point)
        .normalize(),
    });
  }

  return partition;
}

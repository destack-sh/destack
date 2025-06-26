import { getStrokeOutlineTracks } from "@destack-web/shared/freehand/getStrokeOutlinePoints";
import { getStrokePoints } from "@destack-web/shared/freehand/getStrokePoints";
import { setStrokePointRadii } from "@destack-web/shared/freehand/setStrokePointRadii";
import { StrokeOptions, StrokePoint } from "@destack-web/shared/freehand/types";
import { average, precise, toDomPrecision } from "@destack-web/shared/freehand/utils";
import { Vector3 } from "destack";

export function svgInk(rawInputPoints: Vector3[], options: StrokeOptions = {}) {
  const points = getStrokePoints(rawInputPoints, options);
  setStrokePointRadii(points, options);
  const partitions = partitionAtElbows(points);
  let svg = "";
  for (const partition of partitions) {
    svg += renderPartition(partition, options);
  }

  return svg;
}

function partitionAtElbows(points: StrokePoint[]): StrokePoint[][] {
  if (points.length <= 2) return [points];

  const result: StrokePoint[][] = [];
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
      const elbowPoint = {
        ...thisPoint,
        point: thisPoint.input,
      };
      currentPartition.push(elbowPoint);
      result.push(cleanUpPartition(currentPartition));
      currentPartition = [elbowPoint];
      continue;
    }
    currentPartition.push(thisPoint);

    if (dpr > 0.7) {
      // Not an elbow
      continue;
    }

    // so now we have a reasonably acute angle but it might not be an elbow if it's far
    // away from it's neighbors, angular dist is a normalized representation of how far away the point is from it's neighbors
    // (normalized by the radius)
    if (
      (prevPoint.point.distance2(thisPoint.point) + thisPoint.point.distance2(nextPoint.point)) /
        ((prevPoint.radius + thisPoint.radius + nextPoint.radius) / 3) ** 2 <
      1.5
    ) {
      // if this point is kinda close to its neighbors and it has a reasonably
      // acute angle, it's probably a hard elbow
      currentPartition.push(thisPoint);
      result.push(cleanUpPartition(currentPartition));
      currentPartition = [thisPoint];
      continue;
    }
  }
  currentPartition.push(points[points.length - 1]);
  result.push(cleanUpPartition(currentPartition));

  return result;
}

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
    partition[0] = {
      ...partition[0],
      vector: partition[0].point.sub(partition[1].point).normalize(),
    };
    partition[partition.length - 1] = {
      ...partition[partition.length - 1],
      vector: partition[partition.length - 2].point
        .sub(partition[partition.length - 1].point)
        .normalize(),
    };
  }
  return partition;
}

function circlePath(cx: number, cy: number, r: number) {
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

function renderPartition(strokePoints: StrokePoint[], options: StrokeOptions = {}): string {
  if (strokePoints.length === 0) return "";
  if (strokePoints.length === 1) {
    return circlePath(strokePoints[0].point.x, strokePoints[0].point.y, strokePoints[0].radius);
  }

  const { left, right } = getStrokeOutlineTracks(strokePoints, options);
  right.reverse();
  let svg = `M${precise(left[0])}T`;

  // draw left track
  for (let i = 1; i < left.length; i++) {
    svg += average(left[i - 1], left[i]);
  }
  // draw end cap arc
  {
    const point = strokePoints[strokePoints.length - 1];
    const radius = point.radius;
    const direction = point.vector.per().mul(-1);
    const arcStart = point.point.add(direction.mul(radius));
    const arcEnd = point.point.add(direction.mul(-radius));
    svg += `${precise(arcStart)}A${toDomPrecision(radius)},${toDomPrecision(
      radius,
    )} 0 0 1 ${precise(arcEnd)}T`;
  }
  // draw right track
  for (let i = 1; i < right.length; i++) {
    svg += average(right[i - 1], right[i]);
  }
  // draw start cap arc
  {
    const point = strokePoints[0];
    const radius = point.radius;
    const direction = point.vector.per();
    const arcStart = point.point.add(direction.mul(radius));
    const arcEnd = point.point.add(direction.mul(-radius));
    svg += `${precise(arcStart)}A${toDomPrecision(radius)},${toDomPrecision(
      radius,
    )} 0 0 1 ${precise(arcEnd)}Z`;
  }
  return svg;
}

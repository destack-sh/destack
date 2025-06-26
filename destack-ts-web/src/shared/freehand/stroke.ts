import { setStrokePointRadii } from "@destack-web/shared/freehand/radius";
import type { StrokeOptions, StrokePoint } from "@destack-web/shared/freehand/types";
import { Vector3 } from "destack";

// browser strokes seem to be off if PI is regular, a tiny offset seems to fix it
const FIXED_PI = Math.PI + 0.0001;
const MIN_START_PRESSURE = 0.025;
const MIN_END_PRESSURE = 0.01;

/**
 * Get left and right outline tracks for a stroke.
 * Returns separate arrays for left and right side points.
 */
export function getStrokeOutlineTracks(
  strokePoints: StrokePoint[],
  options: StrokeOptions = {},
): { left: Vector3[]; right: Vector3[] } {
  const { size = 16, smoothing = 0.5 } = options;

  // can't do anything with an empty array or a stroke with negative size
  if (strokePoints.length === 0 || size <= 0) {
    return { left: [], right: [] };
  }

  const firstStrokePoint = strokePoints[0];
  const lastStrokePoint = strokePoints[strokePoints.length - 1];
  const totalLength = lastStrokePoint.runningLength;
  const minDistance2 = Math.pow(size * smoothing, 2);
  const leftPts: Vector3[] = [];
  const rightPts: Vector3[] = [];

  let prevVector = strokePoints[0].direction;
  let pl = strokePoints[0].point;
  let pr = pl;
  let templ = pl;
  let tempr = pr;

  // keep track of whether the previous point is a sharp corner
  // so that we don't detect the same corner twice
  let isPrevPointSharpCorner = false;

  // find the outline's left and right points
  // iterate through the points and populate the rightPts and leftPts arrays,
  // skipping the first and last points, which will get caps later on
  let strokePoint: StrokePoint;

  for (let i = 0; i < strokePoints.length; i++) {
    strokePoint = strokePoints[i];
    const { point, direction } = strokePoints[i];

    // handle sharp corners
    // find the difference (dot product) between the current and next vector
    // if the next vector is at more than a right angle to the current vector,
    // draw a cap at the current point
    const prevDot = strokePoint.direction.dot(prevVector);
    const nextVector = (i < strokePoints.length - 1 ? strokePoints[i + 1] : strokePoints[i])
      .direction;
    const nextDot = i < strokePoints.length - 1 ? nextVector.dot(strokePoint.direction) : 1;

    const isPointSharpCorner = prevDot < 0 && !isPrevPointSharpCorner;
    const isNextPointSharpCorner = nextDot !== null && nextDot < 0.2;

    if (isPointSharpCorner || isNextPointSharpCorner) {
      // it's a sharp corner - draw a rounded cap and move on to the next point
      // NOTE :Cleanup: consider saving these and drawing them later?
      //  (so that we can avoid crossing future points)
      if (nextDot > -0.62 && totalLength - strokePoint.runningLength > strokePoint.radius) {
        // draw a "soft" corner
        const offset = prevVector.mul(strokePoint.radius);
        const cpr = prevVector.dot(nextVector);
        if (cpr < 0) {
          templ = point.add(offset);
          tempr = point.sub(offset);
        } else {
          templ = point.sub(offset);
          tempr = point.add(offset);
        }
        leftPts.push(templ);
        rightPts.push(tempr);
      } else {
        // draw a "sharp" corner
        const offset = prevVector.mul(strokePoint.radius).per();
        const start = strokePoint.input.sub(offset);
        for (let step = 1 / 13, t = 0; t < 1; t += step) {
          templ = start.rotWith(strokePoint.input, FIXED_PI * t);
          leftPts.push(templ);
          tempr = start.rotWith(strokePoint.input, FIXED_PI + FIXED_PI * -t);
          rightPts.push(tempr);
        }
      }

      pl = templ;
      pr = tempr;

      if (isNextPointSharpCorner) {
        isPrevPointSharpCorner = true;
      }

      continue;
    }

    isPrevPointSharpCorner = false;

    if (strokePoint === firstStrokePoint || strokePoint === lastStrokePoint) {
      const offset = direction.per().mul(strokePoint.radius);
      leftPts.push(point.sub(offset));
      rightPts.push(point.add(offset));
      continue;
    }

    // add regular points
    // Project points to either side of the current point, using the calculated size as a distance.
    // If a point's distance to the previous point on that side greater than the minimum distance
    // (or if the corner is kinda sharp), add the points to the side's points array.
    const offset = nextVector.lerp(direction, nextDot).per().mul(strokePoint.radius);
    templ = point.sub(offset);
    if (i <= 1 || pl.distance2(templ) > minDistance2) {
      leftPts.push(templ);
      pl = templ;
    }
    tempr = point.add(offset);
    if (i <= 1 || pr.distance2(tempr) > minDistance2) {
      rightPts.push(tempr);
      pr = tempr;
    }

    prevVector = direction;
    continue;
  }

  return { left: leftPts, right: rightPts };
}

/**
 * Get an array of points representing the outline of a stroke.
 * Returns points in correct winding order for drawing.
 */
export function getStrokeOutlinePoints(
  strokePoints: StrokePoint[],
  options: StrokeOptions = {},
): Vector3[] {
  const { size = 16, start = {}, end = {}, last: isComplete = false } = options;

  const { cap: capStart = true } = start;
  const { cap: capEnd = true } = end;

  // we can't do anything with an empty array or a stroke with negative size
  if (strokePoints.length === 0 || size <= 0) {
    return [];
  }

  const firstStrokePoint = strokePoints[0];
  const lastStrokePoint = strokePoints[strokePoints.length - 1];
  const totalLength = lastStrokePoint.runningLength;

  const taperStart =
    start.taper === false
      ? 0
      : start.taper === true
        ? Math.max(size, totalLength)
        : (start.taper as number);

  const taperEnd =
    end.taper === false
      ? 0
      : end.taper === true
        ? Math.max(size, totalLength)
        : (end.taper as number);

  // get our collected left and right points
  const { left: leftPts, right: rightPts } = getStrokeOutlineTracks(strokePoints, options);

  // drawing caps
  // Now that we have our points on either side of the line, we need to draw caps at the start and end.
  // Tapered lines don't have caps, but may have dots for very short lines.
  const firstPoint = firstStrokePoint.point;

  const lastPoint =
    strokePoints.length > 1
      ? strokePoints[strokePoints.length - 1].point
      : firstStrokePoint.point.add(new Vector3({ x: 1, y: 1, z: 0 }));

  // draw a dot for very short or completed strokes
  // If the line is too short to gather left or right points and if the line is not tapered on either side,
  // draw a dot. If the line is tapered, then only draw a dot if the line is both very short and complete.
  // If we draw a dot, we can just return those points.
  if (strokePoints.length === 1) {
    if (!(taperStart || taperEnd) || isComplete) {
      const start = firstPoint.add(
        firstPoint.sub(lastPoint).normalize().per().mul(-firstStrokePoint.radius),
      );
      const dotPts: Vector3[] = [];
      for (let step = 1 / 13, t = step; t <= 1; t += step) {
        dotPts.push(start.rotWith(firstPoint, FIXED_PI * 2 * t));
      }
      return dotPts;
    }
  }

  // draw a start cap
  // Unless the line has a tapered start, or unless the line has a tapered end and the line is very short,
  // draw a start cap around the first point. Use the distance between the second left and right point for
  // the cap's radius. Finally remove the first left and right points.
  const startCap: Vector3[] = [];
  if (taperStart || (taperEnd && strokePoints.length === 1)) {
    // the start point is tapered, noop
  } else if (capStart) {
    // draw the round cap - add thirteen points rotating the right point around the start point to the left point
    for (let step = 1 / 8, t = step; t <= 1; t += step) {
      const pt = rightPts[0].rotWith(firstPoint, FIXED_PI * t);
      startCap.push(pt);
    }
  } else {
    // draw the flat cap - add a point to the left and right of the start point
    const cornersVector = leftPts[0].sub(rightPts[0]);
    const offsetA = cornersVector.mul(0.5);
    const offsetB = cornersVector.mul(0.51);

    startCap.push(
      firstPoint.sub(offsetA),
      firstPoint.sub(offsetB),
      firstPoint.add(offsetB),
      firstPoint.add(offsetA),
    );
  }

  // draw an end cap
  // If the line does not have a tapered end, and unless the line has a tapered start and the line is very short,
  // draw a cap around the last point. Finally, remove the last left and right points. Otherwise, add the last point.
  // Note that this cap is a full-turn-and-a-half: this prevents incorrect caps on sharp end turns.
  const endCap: Vector3[] = [];
  const direction = lastStrokePoint.direction.per().neg();

  if (taperEnd || (taperStart && strokePoints.length === 1)) {
    // tapered end - push the last point to the line
    endCap.push(lastPoint);
  } else if (capEnd) {
    // draw the round end cap
    const start = lastPoint.add(direction.mul(lastStrokePoint.radius));
    for (let step = 1 / 29, t = step; t < 1; t += step) {
      endCap.push(start.rotWith(lastPoint, FIXED_PI * 3 * t));
    }
  } else {
    // draw the flat end cap
    endCap.push(
      lastPoint.add(direction.mul(lastStrokePoint.radius)),
      lastPoint.add(direction.mul(lastStrokePoint.radius * 0.99)),
      lastPoint.sub(direction.mul(lastStrokePoint.radius * 0.99)),
      lastPoint.sub(direction.mul(lastStrokePoint.radius)),
    );
  }

  // Return the points in the correct winding order:
  //  1. begin on the left side,
  //  2. then continue around the end cap,
  //  3. then come back along the right side,
  //  4. and finally complete the start cap.
  return leftPts.concat(endCap, rightPts.reverse(), startCap);
}

/**
 * Get an array of stroke points with computed properties.
 * Transform raw input points into stroke points with pressure, vectors, and distances.
 */
export function getStrokePoints(
  rawInputPoints: Vector3[],
  options: StrokeOptions = {},
): StrokePoint[] {
  const { streamline = 0.5, size = 16, simulatePressure = false } = options;

  // if we don't have any points, return an empty array
  if (rawInputPoints.length === 0) return [];

  // find the interpolation level between points
  const t = 0.15 + (1 - streamline) * 0.85;

  // whatever the input is, make sure that the points are in Vector3[]
  let pts = rawInputPoints;
  let pointsRemovedFromNearEnd = 0;

  if (!simulatePressure) {
    // strip low pressure points from the start of the array
    let pt = pts[0];
    while (pt) {
      if (pt.z >= MIN_START_PRESSURE) break;
      pts.shift();
      pt = pts[0];
    }
  }

  if (!simulatePressure) {
    // strip low pressure points from the end of the array
    let pt = pts[pts.length - 1];
    while (pt) {
      if (pt.z >= MIN_END_PRESSURE) break;
      pts.pop();
      pt = pts[pts.length - 1];
    }
  }

  if (pts.length === 0)
    return [
      {
        point: rawInputPoints[0],
        input: rawInputPoints[0],
        pressure: simulatePressure ? 0.5 : 0.15,
        direction: new Vector3({ x: 1, y: 1, z: 0 }),
        distance: 0,
        runningLength: 0,
        radius: 1,
      },
    ];

  // strip points that are too close to the first point
  let pt = pts[1];
  while (pt) {
    if (pt.distance2(pts[0]) > (size / 3) ** 2) break;
    pts[0] = new Vector3({ x: pts[0].x, y: pts[0].y, z: Math.max(pts[0].z, pt.z) }); // use maximum pressure
    pts.splice(1, 1);
    pt = pts[1];
  }

  // strip points that are too close to the last point
  const last = pts.pop()!;
  pt = pts[pts.length - 1];
  while (pt) {
    if (pt.distance2(last) > (size / 3) ** 2) break;
    pts.pop();
    pt = pts[pts.length - 1];
    pointsRemovedFromNearEnd++;
  }
  pts.push(last);

  const isComplete =
    options.last ||
    !options.simulatePressure ||
    (pts.length > 1 && pts[pts.length - 1].distance2(pts[pts.length - 2]) < size ** 2) ||
    pointsRemovedFromNearEnd > 0;

  // add extra points between the two, to help avoid "dash" lines
  // for strokes with tapered start and ends. don't mutate the input array!
  if (pts.length === 2 && options.simulatePressure) {
    const last = pts[1];
    pts = pts.slice(0, -1);
    for (let i = 1; i < 5; i++) {
      let next = pts[0].lerp(last, i / 4);
      next = new Vector3({
        x: next.x,
        y: next.y,
        z: ((pts[0].z + (last.z - pts[0].z)) * i) / 4,
      });
      pts.push(next);
    }
  }

  // the strokePoints array will hold the points for the stroke
  // start it out with the first point, which needs no adjustment
  const strokePoints: StrokePoint[] = [
    {
      point: pts[0],
      input: pts[0],
      pressure: simulatePressure ? 0.5 : pts[0].z,
      direction: new Vector3({ x: 1, y: 1, z: 0 }),
      distance: 0,
      runningLength: 0,
      radius: 1,
    },
  ];

  // we use the totalLength to keep track of the total distance
  let totalLength = 0;

  // we're set this to the latest point, so we can use it to calculate
  // the distance and vector of the next point
  let prev = strokePoints[0];

  // iterate through all of the points, creating StrokePoints
  let point: Vector3, distance: number;

  if (isComplete && streamline > 0) {
    pts.push(pts[pts.length - 1]);
  }

  for (let i = 1, n = pts.length; i < n; i++) {
    point = !t || (options.last && i === n - 1) ? pts[i] : pts[i].lerp(prev.point, 1 - t);

    // if the new point is the same as the previous point, skip ahead
    if (prev.point.equals(point)) continue;

    distance = point.distance(prev.point);
    totalLength += distance;

    // at the start of the line, we wait until the new point is a
    // certain distance away from the original point, to avoid noise
    if (i < 4 && totalLength < size) {
      continue;
    }

    // create a new strokepoint (it will be the new "previous" one)
    prev = {
      input: pts[i],
      // the adjusted point
      point,
      // the input pressure (or .5 if not specified)
      pressure: simulatePressure ? 0.5 : pts[i].z,
      // the vector from the current point to the previous point
      direction: prev.point.sub(point).normalize(),
      // the distance between the current point and the previous point
      distance,
      // the total distance so far
      runningLength: totalLength,
      // the stroke point's radius
      radius: 1,
    };

    // push it to the strokePoints array
    strokePoints.push(prev);
  }

  // set the vector of the first point to be the same as the second point
  if (strokePoints[1]?.direction) {
    strokePoints[0].direction = strokePoints[1].direction;
  }

  if (totalLength < 1) {
    const maxPressureAmongPoints = Math.max(0.5, ...strokePoints.map((s) => s.pressure));
    strokePoints.forEach((s) => (s.pressure = maxPressureAmongPoints));
  }

  return strokePoints;
}

/**
 * Get an array of points describing a polygon that surrounds the input points.
 * This is the main entry point that combines all stroke processing steps.
 */
export function getStroke(points: Vector3[], options: StrokeOptions = {}): Vector3[] {
  return getStrokeOutlinePoints(
    setStrokePointRadii(getStrokePoints(points, options), options),
    options,
  );
}

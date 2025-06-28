import { EASING_FUNCTIONS } from "@destack-web/shared/easings";
import { Stroke, StrokePoint, Vector2 } from "destack";

const RATE_OF_PRESSURE_CHANGE = 0.275;

// browser strokes seem to be off if PI is regular, a tiny offset seems to fix it
const FIXED_PI = Math.PI + 0.0001;
const SIMULATED_PRESSURE = 0.5;

/**
 * Get an array of points describing a polygon that surrounds the input points.
 */
export function getStroke(
  points: Vector2[],
  stroke: Stroke,
  options: { isComplete: boolean },
): Vector2[] {
  const strokePoints = getStrokePoints(points, stroke, options);
  return getStrokeOutlinePoints(strokePoints, stroke, options);
}

/**
 * Get left and right outline tracks for a stroke.
 */
export function getStrokeOutlineTracks(
  points: readonly StrokePoint[],
  stroke: Stroke,
): { left: Vector2[]; right: Vector2[] } {
  const { size = 16, smoothing = 0.5 } = stroke;

  // can't do anything with an empty array or a stroke with negative size
  if (points.length === 0 || size <= 0) {
    return { left: [], right: [] };
  }

  const firstStrokePoint = points[0];
  const lastStrokePoint = points[points.length - 1];
  const totalLength = lastStrokePoint.runningLength;
  const minDistance2 = Math.pow(size * smoothing, 2);
  const leftPoints: Vector2[] = [];
  const rightPoints: Vector2[] = [];

  let prevVector = points[0].direction;
  let pl = points[0].point;
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

  for (let i = 0; i < points.length; i++) {
    strokePoint = points[i];
    const { point, direction } = points[i];

    // handle sharp corners
    // find the difference (dot product) between the current and next vector
    // if the next vector is at more than a right angle to the current vector,
    // draw a cap at the current point
    const prevDot = strokePoint.direction.dot(prevVector);
    const nextVector = (i < points.length - 1 ? points[i + 1] : points[i]).direction;
    const nextDot = i < points.length - 1 ? nextVector.dot(strokePoint.direction) : 1;

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
        leftPoints.push(templ);
        rightPoints.push(tempr);
      } else {
        // draw a "sharp" corner
        const offset = prevVector.mul(strokePoint.radius).per();
        const start = strokePoint.originalPoint.sub(offset);
        for (let step = 1 / 13, t = 0; t < 1; t += step) {
          templ = start.rotWith(strokePoint.originalPoint, FIXED_PI * t);
          leftPoints.push(templ);
          tempr = start.rotWith(strokePoint.originalPoint, FIXED_PI + FIXED_PI * -t);
          rightPoints.push(tempr);
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
      leftPoints.push(point.sub(offset));
      rightPoints.push(point.add(offset));
      continue;
    }

    // add regular points
    // Project points to either side of the current point, using the calculated size as a distance.
    // If a point's distance to the previous point on that side greater than the minimum distance
    // (or if the corner is kinda sharp), add the points to the side's points array.
    const offset = nextVector.lerp(direction, nextDot).per().mul(strokePoint.radius);
    templ = point.sub(offset);
    if (i <= 1 || pl.distance2(templ) > minDistance2) {
      leftPoints.push(templ);
      pl = templ;
    }
    tempr = point.add(offset);
    if (i <= 1 || pr.distance2(tempr) > minDistance2) {
      rightPoints.push(tempr);
      pr = tempr;
    }

    prevVector = direction;
    continue;
  }

  return { left: leftPoints, right: rightPoints };
}

/**
 * Get an array of points representing the outline of a stroke.
 * Returns points in correct winding order for drawing.
 */
export function getStrokeOutlinePoints(
  points: StrokePoint[],
  stroke: Stroke,
  options: { isComplete: boolean },
): Vector2[] {
  const { size, start, end } = stroke;

  const capStart = start?.cap == null ? true : start.cap;
  const capEnd = end?.cap == null ? true : end.cap;

  // we can't do anything with an empty array or a stroke with negative size
  if (points.length === 0 || size <= 0) {
    return [];
  }

  const firstStrokePoint = points[0];
  const lastStrokePoint = points[points.length - 1];
  const totalLength = lastStrokePoint.runningLength;
  const taperStart = start?.taper == null ? 0 : start.taper;
  const taperEnd = end?.taper == null ? 0 : end.taper;

  // get our collected left and right points
  const { left: leftPts, right: rightPts } = getStrokeOutlineTracks(points, stroke);

  // drawing caps
  // Now that we have our points on either side of the line, we need to draw caps at the start and end.
  // Tapered lines don't have caps, but may have dots for very short lines.
  const firstPoint = firstStrokePoint.point;
  const lastPoint =
    points.length > 1
      ? points[points.length - 1].point
      : firstStrokePoint.point.add(new Vector2({ x: 1, y: 1 }));

  // draw a dot for very short or completed strokes
  // If the line is too short to gather left or right points and if the line is not tapered on either side,
  // draw a dot. If the line is tapered, then only draw a dot if the line is both very short and complete.
  // If we draw a dot, we can just return those points.
  if (points.length === 1) {
    if (!(taperStart || taperEnd) || options.isComplete) {
      const start = firstPoint.add(
        firstPoint.sub(lastPoint).normalize().per().mul(-firstStrokePoint.radius),
      );
      const dotPts: Vector2[] = [];
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
  const startCap: Vector2[] = [];
  if (taperStart || (taperEnd && points.length === 1)) {
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
  const endCap: Vector2[] = [];
  const direction = lastStrokePoint.direction.per().neg();

  if (taperEnd || (taperStart && points.length === 1)) {
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

  // return the points in the correct winding order:
  //  1. begin on the left side,
  //  2. then continue around the end cap,
  //  3. then come back along the right side,
  //  4. and finally complete the start cap.
  return leftPts.concat(endCap, rightPts.reverse(), startCap);
}

/**
 * Get an array of renderable StrokePoints from raw Vector2 points.
 */
export function getStrokePoints(
  points: readonly Vector2[],
  stroke: Stroke,
  options: { isComplete: boolean },
): StrokePoint[] {
  if (points.length === 0) return [];

  const { streamline, size, thinning, easing, start, end } = stroke;
  const t = 0.15 + (1 - streamline) * 0.85;
  let pts = points.slice();
  let pointsRemovedFromNearEnd = 0;

  if (pts.length === 0)
    return [
      new StrokePoint({
        point: points[0],
        originalPoint: points[0],
        pressure: SIMULATED_PRESSURE,
        direction: new Vector2({ x: 1, y: 1 }),
        distance: 0,
        runningLength: 0,
        radius: 1,
      }),
    ];

  // strip points that are too close to the first point
  let pt = pts[1];
  while (pt) {
    if (pt.distance2(pts[0]) > (size / 3) ** 2) break;
    pts[0] = new Vector2({ x: pts[0].x, y: pts[0].y });
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
    options.isComplete ||
    (pts.length > 1 && pts[pts.length - 1].distance2(pts[pts.length - 2]) < size ** 2) ||
    pointsRemovedFromNearEnd > 0;

  // add extra points between the two,
  // (to help avoid "dash" lines for strokes with tapered start and ends)
  if (pts.length === 2) {
    const last = pts[1];
    pts = pts.slice(0, -1);
    for (let i = 1; i < 5; i++) {
      let next = pts[0].lerp(last, i / 4);
      next = new Vector2({ x: next.x, y: next.y });
      pts.push(next);
    }
  }

  // the strokePoints array will hold the points for the stroke
  // start it out with the first point, which needs no adjustment
  let strokePoints: StrokePoint[] = [
    new StrokePoint({
      point: pts[0],
      originalPoint: pts[0],
      pressure: SIMULATED_PRESSURE,
      direction: new Vector2({ x: 1, y: 1 }),
      distance: 0,
      runningLength: 0,
      radius: 1,
    }),
  ];

  let totalLength = 0;
  let prevPoint = strokePoints[0];
  let point: Vector2;
  let distance: number;

  if (isComplete && streamline > 0) {
    pts.push(pts[pts.length - 1]);
  }

  for (let i = 1, n = pts.length; i < n; i++) {
    if (!t || (options.isComplete && i === n - 1)) {
      point = pts[i];
    } else {
      point = pts[i].lerp(prevPoint.point, 1 - t);
    }

    // ignore duplicate points
    if (prevPoint.point.x === point.x && prevPoint.point.y === point.y) {
      continue;
    }

    distance = point.distance(prevPoint.point);
    totalLength += distance;

    // at the start of the line, wait until the new point is a
    // certain distance away from the original point to avoid noise
    if (i < 4 && totalLength < size) {
      continue;
    }

    // new strokepoint
    prevPoint = new StrokePoint({
      originalPoint: pts[i],
      point,
      pressure: SIMULATED_PRESSURE,
      direction: prevPoint.point.sub(point).normalize(),
      distance,
      runningLength: totalLength,
      radius: 1,
    });
    strokePoints.push(prevPoint);
  }

  // set the vector of the first point to be the same as the second point
  if (strokePoints[1]?.direction) {
    strokePoints[0] = new StrokePoint({
      ...strokePoints[0],
      direction: strokePoints[1].direction,
    });
  }

  // for very short strokes, set the pressure to the maximum pressure among all points
  if (totalLength < 1) {
    const maxPressureAmongPoints = Math.max(
      SIMULATED_PRESSURE,
      ...strokePoints.map((s) => s.pressure),
    );
    strokePoints = strokePoints.map(
      (s) =>
        new StrokePoint({
          ...s,
          pressure: maxPressureAmongPoints,
        }),
    );
  }

  // calculate and set radius values for stroke points
  // apply pressure-based thinning and tapering effects
  const taperStartEase = start?.easing;
  const taperEndEase = end?.easing;
  const easingFunction = EASING_FUNCTIONS[easing];
  const taperStartEaseFunction = taperStartEase ? EASING_FUNCTIONS[taperStartEase] : undefined;
  const taperEndEaseFunction = taperEndEase ? EASING_FUNCTIONS[taperEndEase] : undefined;

  let firstRadius: number | undefined;
  let prevPressure = strokePoints[0].pressure;
  let strokePoint: StrokePoint;

  // calculate initial pressure based on average of first n points
  // this prevents "dots" at the start of drawn lines
  let p: number;
  for (let i = 0, n = strokePoints.length; i < n; i++) {
    strokePoint = strokePoints[i];
    if (strokePoint.runningLength > size * 5) break;

    const sp = Math.min(1, strokePoint.distance / size);
    const rp = Math.min(1, 1 - sp);
    p = Math.min(1, prevPressure + (rp - prevPressure) * (sp * RATE_OF_PRESSURE_CHANGE));
    prevPressure = prevPressure + (p - prevPressure) * 0.5;
  }

  // calculate pressure and radius for each point
  for (let i = 0; i < strokePoints.length; i++) {
    strokePoint = strokePoints[i];

    if (thinning) {
      let { pressure } = strokePoint;
      const sp = Math.min(1, strokePoint.distance / size);
      const rp = Math.min(1, 1 - sp);
      pressure = Math.min(1, prevPressure + (rp - prevPressure) * (sp * RATE_OF_PRESSURE_CHANGE));
      const radius = size * easingFunction(0.5 - thinning * (0.5 - pressure));
      strokePoints[i] = new StrokePoint({ ...strokePoint, radius });
      prevPressure = pressure;
    } else {
      strokePoints[i] = new StrokePoint({ ...strokePoint, radius: size / 2 });
    }

    if (firstRadius === undefined) {
      firstRadius = strokePoints[i].radius;
    }
  }

  // apply tapering at start and end
  const taperStart = start?.taper ? Math.max(size, totalLength) : 0;
  const taperEnd = end?.taper ? Math.max(size, totalLength) : 0;
  if (taperStart > 0 || taperEnd > 0) {
    for (let i = 0; i < strokePoints.length; i++) {
      strokePoint = strokePoints[i];
      const { runningLength } = strokePoint;
      const taperStartFactor =
        typeof taperStart === "number" && runningLength < taperStart && taperStartEaseFunction
          ? taperStartEaseFunction(runningLength / taperStart)
          : 1;
      const taperEndFactor =
        typeof taperEnd === "number" &&
        totalLength - runningLength < taperEnd &&
        taperEndEaseFunction
          ? taperEndEaseFunction((totalLength - runningLength) / taperEnd)
          : 1;
      strokePoints[i] = new StrokePoint({
        ...strokePoint,
        radius: Math.max(0.01, strokePoint.radius * Math.min(taperStartFactor, taperEndFactor)),
      });
    }
  }

  return strokePoints;
}

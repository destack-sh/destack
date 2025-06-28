import { StrokeOptions, StrokePoint } from "@destack-web/shared/freehand/types";

const RATE_OF_PRESSURE_CHANGE = 0.275;

/**
 * Calculate and set radius values for stroke points.
 * Apply pressure-based thinning and tapering effects.
 */
export function setStrokePointRadii(
  strokePoints: StrokePoint[],
  options: StrokeOptions,
): StrokePoint[] {
  const { size, thinning, simulatePressure, easing, start, end } = options;
  const { easing: taperStartEase } = start;
  const { easing: taperEndEase } = end;

  const totalLength = strokePoints[strokePoints.length - 1].runningLength;

  let firstRadius: number | undefined;
  let prevPressure = strokePoints[0].pressure;
  let strokePoint: StrokePoint;

  // handle very short strokes without pressure simulation
  if (!simulatePressure && totalLength < size) {
    const max = strokePoints.reduce((max, curr) => Math.max(max, curr.pressure), 0.5);
    strokePoints.forEach((sp) => {
      sp.pressure = max;
      sp.radius = size * easing(0.5 - thinning * (0.5 - sp.pressure));
    });
    return strokePoints;
  }

  // calculate initial pressure based on average of first n points
  // this prevents "dots" at the start of drawn lines
  let p: number;
  for (let i = 0, n = strokePoints.length; i < n; i++) {
    strokePoint = strokePoints[i];
    if (strokePoint.runningLength > size * 5) break;

    const sp = Math.min(1, strokePoint.distance / size);
    if (simulatePressure) {
      const rp = Math.min(1, 1 - sp);
      p = Math.min(1, prevPressure + (rp - prevPressure) * (sp * RATE_OF_PRESSURE_CHANGE));
    } else {
      p = Math.min(1, prevPressure + (strokePoint.pressure - prevPressure) * 0.5);
    }
    prevPressure = prevPressure + (p - prevPressure) * 0.5;
  }

  // calculate pressure and radius for each point
  for (let i = 0; i < strokePoints.length; i++) {
    strokePoint = strokePoints[i];

    if (thinning) {
      let { pressure } = strokePoint;
      const sp = Math.min(1, strokePoint.distance / size);

      if (simulatePressure) {
        // simulate pressure based on distance and stroke size
        const rp = Math.min(1, 1 - sp);
        pressure = Math.min(1, prevPressure + (rp - prevPressure) * (sp * RATE_OF_PRESSURE_CHANGE));
      } else {
        // use input pressure with light smoothing
        pressure = Math.min(
          1,
          prevPressure + (pressure - prevPressure) * (sp * RATE_OF_PRESSURE_CHANGE),
        );
      }

      strokePoint.radius = size * easing(0.5 - thinning * (0.5 - pressure));
      prevPressure = pressure;
    } else {
      strokePoint.radius = size / 2;
    }

    if (firstRadius === undefined) {
      firstRadius = strokePoint.radius;
    }
  }

  // apply tapering at start and end
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
  if (taperStart || taperEnd) {
    for (let i = 0; i < strokePoints.length; i++) {
      strokePoint = strokePoints[i];
      const { runningLength } = strokePoint;
      const taperStartFactor =
        runningLength < taperStart ? taperStartEase(runningLength / taperStart) : 1;
      const taperEndFactor =
        totalLength - runningLength < taperEnd
          ? taperEndEase((totalLength - runningLength) / taperEnd)
          : 1;
      strokePoint.radius = Math.max(
        0.01,
        strokePoint.radius * Math.min(taperStartFactor, taperEndFactor),
      );
    }
  }

  return strokePoints;
}

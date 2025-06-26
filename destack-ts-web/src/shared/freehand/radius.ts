import { EASINGS } from "@destack-web/shared/easings";
import { StrokeOptions, StrokePoint } from "@destack-web/shared/freehand/types";

const { min } = Math;

// rate of change for simulated pressure
const RATE_OF_PRESSURE_CHANGE = 0.275;

/**
 * Calculate and set radius values for stroke points.
 * Apply pressure-based thinning and tapering effects.
 */
export function setStrokePointRadii(
  strokePoints: StrokePoint[],
  options: StrokeOptions,
): StrokePoint[] {
  const {
    size = 16,
    thinning = 0.5,
    simulatePressure = true,
    easing = (t) => t,
    start = {},
    end = {},
  } = options;

  const { easing: taperStartEase = EASINGS.easeOutQuad } = start;
  const { easing: taperEndEase = EASINGS.easeOutCubic } = end;

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
    
    const sp = min(1, strokePoint.distance / size);
    if (simulatePressure) {
      const rp = min(1, 1 - sp);
      p = min(1, prevPressure + (rp - prevPressure) * (sp * RATE_OF_PRESSURE_CHANGE));
    } else {
      p = min(1, prevPressure + (strokePoint.pressure - prevPressure) * 0.5);
    }
    prevPressure = prevPressure + (p - prevPressure) * 0.5;
  }

  // calculate pressure and radius for each point
  for (let i = 0; i < strokePoints.length; i++) {
    strokePoint = strokePoints[i];
    
    if (thinning) {
      let { pressure } = strokePoint;
      const sp = min(1, strokePoint.distance / size);
      
      if (simulatePressure) {
        // simulate pressure based on distance and stroke size
        const rp = min(1, 1 - sp);
        pressure = min(1, prevPressure + (rp - prevPressure) * (sp * RATE_OF_PRESSURE_CHANGE));
      } else {
        // use input pressure with light smoothing
        pressure = min(
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

      // calculate taper strength at start
      const ts = runningLength < taperStart ? taperStartEase(runningLength / taperStart) : 1;

      // calculate taper strength at end
      const te =
        totalLength - runningLength < taperEnd
          ? taperEndEase((totalLength - runningLength) / taperEnd)
          : 1;

      // apply the smaller of the two taper strengths
      strokePoint.radius = Math.max(0.01, strokePoint.radius * Math.min(ts, te));
    }
  }

  return strokePoints;
}

/**
 * Compute radius based on pressure.
 */
export function getStrokeRadius(
  size: number,
  thinning: number,
  pressure: number,
  easing: (t: number) => number = (t) => t,
) {
  return size * easing(0.5 - thinning * (0.5 - pressure));
}

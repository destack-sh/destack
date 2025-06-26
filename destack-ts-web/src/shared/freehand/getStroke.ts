import { getStrokeOutlinePoints } from "@destack-web/shared/freehand/getStrokeOutlinePoints";
import { getStrokePoints } from "@destack-web/shared/freehand/getStrokePoints";
import { setStrokePointRadii } from "@destack-web/shared/freehand/setStrokePointRadii";
import type { StrokeOptions } from "@destack-web/shared/freehand/types";
import { Vector3 } from "destack";

/**
 * ## getStroke
 *
 * Get an array of points describing a polygon that surrounds the input points.
 *
 * @param points - An array of points (as `[x, y, pressure]` or `{x, y, pressure}`). Pressure is
 *   optional in both cases.
 * @param options - An object with options.
 * @public
 */

export function getStroke(points: Vector3[], options: StrokeOptions = {}): Vector3[] {
  return getStrokeOutlinePoints(
    setStrokePointRadii(getStrokePoints(points, options), options),
    options,
  );
}

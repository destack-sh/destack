import { Vector3 } from "destack";

/** A point along a stroke path with computed properties. */
export interface StrokePoint {
  /** The adjusted point position. */
  point: Vector3;
  
  /** The original input point. */
  input: Vector3;
  
  /** The pressure value at this point (0-1). */
  pressure: number;
  
  /** The normalized direction vector from previous point. */
  direction: Vector3;
  
  /** Distance from the previous point. */
  distance: number;
  
  /** Total distance from stroke start. */
  runningLength: number;
  
  /** The computed radius at this point. */
  radius: number;
}

/** Configuration options for stroke generation. */
export interface StrokeOptions {
  /** Base stroke size/width. */
  size?: number;
  
  /** Amount of pressure-based thinning (0-1). */
  thinning?: number;
  
  /** Amount of path smoothing (0-1). */
  smoothing?: number;
  
  /** Amount of streamlining applied to path (0-1). */
  streamline?: number;
  
  /** Whether to simulate pressure if not provided. */
  simulatePressure?: boolean;
  
  /** Easing function for pressure mapping. */
  easing?: (t: number) => number;
  
  /** Whether this is the final stroke. */
  last?: boolean;
  
  /** Start cap configuration. */
  start?: {
    cap?: boolean;
    taper?: number | boolean;
    easing?: (t: number) => number;
  };
  
  /** End cap configuration. */
  end?: {
    cap?: boolean;
    taper?: number | boolean;
    easing?: (t: number) => number;
  };
}

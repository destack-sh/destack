import { Vector3 } from "destack";

/** @public */
export function precise(A: Vector3) {
  return `${toDomPrecision(A.x)},${toDomPrecision(A.y)} `;
}

/** @public */
export function average(A: Vector3, B: Vector3) {
  return `${toDomPrecision((A.x + B.x) / 2)},${toDomPrecision((A.y + B.y) / 2)} `;
}

/**
 * The DOM likes values to be fixed to 3 decimal places
 *
 * @public
 */
export function toDomPrecision(v: number) {
  return Math.round(v * 1e4) / 1e4;
}

/**
 * @public
 */
export function toFixed(v: number) {
  return Math.round(v * 1e2) / 1e2;
}

import { Vector } from "destack";

export function vector2String(A: Vector) {
  return `${toDomPrecision(A.x)},${toDomPrecision(A.y)} `;
}

export function averageVector2String(A: Vector, B: Vector) {
  return `${toDomPrecision((A.x + B.x) / 2)},${toDomPrecision((A.y + B.y) / 2)} `;
}

export function toDomPrecision(v: number) {
  return Math.round(v * 1e4) / 1e4;
}

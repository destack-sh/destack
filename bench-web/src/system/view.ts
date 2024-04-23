import { Orientation } from "@/proto/wire";

export function getOrientationFlex(orientation: Orientation = Orientation.HORIZONTAL): string {
  return orientation == Orientation.HORIZONTAL ? "flex-row" : "flex-col";
}

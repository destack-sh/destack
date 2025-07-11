import { Value } from "@destack/language";

export function packEventRow(value: Value): { [key: string]: string } {
  throw new Error("Not implemented");
}

export function unpackEventRow(value: { [key: string]: string }): Value {
  throw new Error("Not implemented");
}

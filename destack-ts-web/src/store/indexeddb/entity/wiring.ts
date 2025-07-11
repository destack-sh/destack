import { Value } from "@destack/language";

export function packEntityRow(value: Value): { [key: string]: string } {
  throw new Error("Not implemented");
}

export function unpackEntityRow(value: { [key: string]: string }): Value {
  throw new Error("Not implemented");
}

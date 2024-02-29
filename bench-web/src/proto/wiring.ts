import { BenchType, StructType, type AnyNodeData, type AnyStructData, type StructTypeMapping } from "@/proto/wire";

export function newStructId(): number {
  /** Generates a positive 32-bit random integer */
  return Math.floor(Math.random() * 0x7fffffff);
}

export function makeStruct<T extends StructType>(metatype: T, data: Omit<StructTypeMapping[T], "metatype" | "id">): StructTypeMapping[T] {
  const struct = {
    metatype: metatype as unknown as BenchType,
    id: newStructId(),
    ...data,
  };
  return struct as StructTypeMapping[T];
}

export function toRobustJson(value: AnyStructData | AnyNodeData): { [key: string]: any } {
  throw new Error("not yet implemented");
}
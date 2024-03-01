import {
  BenchType,
  StructType,
  type AnyNodeData,
  type AnyStructData,
  type StructTypeMapping,
  SomeNodeData,
  NodeType,
} from "@/proto/wire";
import { reverseRecord } from "@/utils/functools";
import { Casing, toCasing } from "@/utils/string";

export const NODE_TYPE_NAME: Record<NodeType, string> = reverseRecord(NodeType);
export const STRUCT_TYPE_NAME: Record<StructType, string> = reverseRecord(StructType);
export const BENCH_TYPE_NAME: Record<BenchType, string> = reverseRecord(BenchType);

export function newStructId(): number {
  /** Generates a positive 32-bit random integer */
  return Math.floor(Math.random() * 0x7fffffff);
}

export function makeStruct<T extends StructType>(
  metatype: T,
  data: Omit<StructTypeMapping[T], "metatype" | "id">,
): StructTypeMapping[T] {
  const struct = {
    metatype: metatype as unknown as BenchType,
    id: newStructId(),
    ...data,
  };
  return struct as StructTypeMapping[T];
}

export function toProtoOneOf<T extends object>(value: T): T & { oneofKind: keyof T } {
  /** Turn { [key]: value } into { key: value, oneofKind: key } for protobuf unions */
  const key = Object.keys(value)[0] as keyof T;
  return { ...value, oneofKind: key };
}

export function wrapSomeNode(node: AnyNodeData): SomeNodeData {
  const fieldName = toCasing(BENCH_TYPE_NAME[node.metatype], Casing.SNAKE);
  return { node: { [fieldName]: node, oneofKind: fieldName as any } };
}

export function unwrapSomeNode(node: SomeNodeData): AnyNodeData {
  const oneOfKind = node.node.oneofKind;
  if (!oneOfKind) throw new Error("missing oneofKind");
  return (node.node as any)[oneOfKind];
}

export function toRobustJson(value: AnyStructData | AnyNodeData): { [key: string]: any } {
  throw new Error("not yet implemented");
}

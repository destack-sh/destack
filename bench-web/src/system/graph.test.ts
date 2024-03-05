import {
  BENCH_TYPE_BY_MESSAGE_TYPE_NAME,
  BenchType,
  MESSAGE_TYPE_BY_BENCH_TYPE,
  PROPERTY_ENUM_BY_TYPE,
  StructType,
  type AnyPropertyType,
  type AnyTypeMapping,
  NodeType,
} from "@/proto/wire";
import { InMemoryNodeGraph, toNodeReference } from "@/system/graph";
import { ScalarType } from "@protobuf-ts/runtime";
import { v4 } from "uuid";
import { describe, test } from "vitest";

const SCALAR_GENERATORS: Partial<Record<ScalarType, () => any>> = {
  [ScalarType.DOUBLE]: () => Math.random(),
  [ScalarType.FLOAT]: () => Math.random(),
  [ScalarType.INT32]: () => Math.floor(Math.random() * 0x7fffffff),
  [ScalarType.INT64]: () => Math.floor(Math.random() * 0x7fffffff),
  [ScalarType.UINT32]: () => Math.floor(Math.random() * 0x7fffffff),
  [ScalarType.UINT64]: () => Math.floor(Math.random() * 0x7fffffff),
  [ScalarType.SINT32]: () => Math.floor(Math.random() * 0x7fffffff),
  [ScalarType.SINT64]: () => Math.floor(Math.random() * 0x7fffffff),
  [ScalarType.BOOL]: () => Math.random() > 0.5,
  [ScalarType.STRING]: () => v4(),
  [ScalarType.BYTES]: () => v4(),
};

const MESSAGE_TYPE_GENERATORS: Record<string, () => any> = {
  "google.protobuf.Timestamp": () => new Date().toISOString(),
  "google.protobuf.Duration": () => Math.random(),
  "symbolx.bench.NodeReferenceData": () => ({ metatype: BenchType.NODE_REFERENCE, id: v4(), type: NodeType.USER }),
};

export function fabricate<T extends BenchType>(
  metatype: T,
  options?: { path?: StructType[]; unset?: (keyof AnyTypeMapping[T])[]; set?: Partial<AnyTypeMapping[T]> },
): AnyTypeMapping[T] {
  const allProperties: AnyPropertyType = PROPERTY_ENUM_BY_TYPE[metatype]!;
  const messageType = MESSAGE_TYPE_BY_BENCH_TYPE[metatype]!;
  const struct = { };
  let ord = 0;
  for (const propName of Object.keys(allProperties)) {
    const field = messageType.fields[ord];
    if (!Number.isNaN(Number(propName))) continue; // skip numeric keys
    let value: any;
    if (propName == "metatype") {
      value = metatype;
    } else if (options?.unset?.includes(propName as any)) {
      value = undefined;
    } else if (options?.set != null && (options.set as any)[propName] !== undefined) {
      value = (options.set as any)[propName];
    } else if (field.name.endsWith("id") || field.name.endsWith("ck")) {
      value = v4();
    } else if (field.kind == "scalar" && SCALAR_GENERATORS[field.T] != null) {
      value = SCALAR_GENERATORS[field.T]!();
    } else if (field.kind == "enum") {
      value = field.T()[0];
    } else if (field.kind == "message" && BENCH_TYPE_BY_MESSAGE_TYPE_NAME[field.T().typeName]) {
      const benchType = BENCH_TYPE_BY_MESSAGE_TYPE_NAME[field.T().typeName]!;
      value = fabricate(benchType);
    } else if (field.kind == "message" && MESSAGE_TYPE_GENERATORS[field.T().typeName]) {
      value = MESSAGE_TYPE_GENERATORS[field.T().typeName]!();
    } else {
      throw new Error(`no generator for field ${messageType.typeName}.${propName} [kind=${field.kind}]`);
    }
    (struct as any)[propName] = value;
    ord += 1;
  }
  return struct as AnyTypeMapping[T];
}

test("fabricate", () => fabricate(BenchType.USER, { unset: ["parentPtr"] }));

describe("memory graph", () => {
  const graph = new InMemoryNodeGraph();
  const user = fabricate(BenchType.USER, { unset: ["parentPtr"] });
  const client = fabricate(BenchType.CLIENT, { set: { parentPtr: toNodeReference(user) } });

  test("create", () => {
    graph.extend(user, client);
  });
});

// describe("layered graph", () => {});

// describe("filtered graph", () => {});

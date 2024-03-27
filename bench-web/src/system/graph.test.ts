import {
  BENCH_TYPE_BY_MESSAGE_TYPE_NAME,
  BenchType,
  ClientData,
  ClientProperty,
  MESSAGE_TYPE_BY_BENCH_TYPE,
  NodeType,
  PROPERTY_ENUM_BY_TYPE,
  StructType,
  UserData,
  type AnyPropertyType,
  type AnyTypeMapping,
  BENCH_TYPES,
} from "@/proto/wire";
import { toNodeReference } from "@/proto/wiring";
import { LayerNodeGraph, NodeGraph, ProxyNodeGraph } from "@/system/graph";
import { ScalarType, type FieldInfo } from "@protobuf-ts/runtime";
import { v4 } from "uuid";
import { describe, expect, test } from "vitest";

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
  "google.protobuf.Struct": () => {},
  "google.protobuf.Timestamp": () => new Date().toISOString(),
  "google.protobuf.Duration": () => Math.random(),
  "symbolx.bench.NodeReferenceData": () => ({ metatype: BenchType.NODE_REFERENCE, id: v4(), type: NodeType.USER }),
};

const PROP_NAME_GENERATORS: Record<string, () => any> = {
  valuePacked: () => {},
};

const MEMBERS_BY_ENUM: Record<string, number[]> = {};

export function fabricate<T extends BenchType>(
  metatype: T,
  options?: { path?: BenchType[]; unset?: (keyof AnyTypeMapping[T])[]; set?: Partial<AnyTypeMapping[T]> },
): AnyTypeMapping[T] {
  const allProperties: AnyPropertyType = PROPERTY_ENUM_BY_TYPE[metatype]!;
  const messageType = MESSAGE_TYPE_BY_BENCH_TYPE[metatype]!;

  function fabricateScalarProp(propName: string, field: FieldInfo): any {
    let value: any;
    if (options?.unset?.includes(propName as any)) {
      value = undefined;
    } else if (options?.set != null && (options.set as any)[propName] !== undefined) {
      value = (options.set as any)[propName];
    } else if (field.name.endsWith("id") || field.name.endsWith("ck")) {
      value = v4();
    } else if (field.kind == "scalar" && SCALAR_GENERATORS[field.T] != null) {
      value = SCALAR_GENERATORS[field.T]!();
    } else if (field.kind == "enum") {
      const [typeName, enu] = field.T();
      if (MEMBERS_BY_ENUM[typeName] == null) {
        MEMBERS_BY_ENUM[typeName] = Object.keys(enu)
          .map((n) => Number(n))
          .filter((n) => !isNaN(n));
      }
      const members = MEMBERS_BY_ENUM[typeName];
      value = members[Math.floor(Math.random() * members.length)];
    } else if (field.kind == "message" && BENCH_TYPE_BY_MESSAGE_TYPE_NAME[field.T().typeName]) {
      const benchType = BENCH_TYPE_BY_MESSAGE_TYPE_NAME[field.T().typeName]!;
      if (options?.path?.includes(benchType)) {
        value = null;
      } else {
        const path = (options?.path ?? []).concat(metatype);
        value = fabricate(benchType, { path });
      }
    } else if (field.kind == "message" && MESSAGE_TYPE_GENERATORS[field.T().typeName]) {
      value = MESSAGE_TYPE_GENERATORS[field.T().typeName]!();
    } else if (PROP_NAME_GENERATORS[propName]) {
      value = PROP_NAME_GENERATORS[propName]!();
    } else {
      throw new Error(`no generator for field ${messageType.typeName}.${propName} [kind=${field.kind}]`);
    }
    return value;
  }

  const struct = {};
  let ord = 0;
  for (const propName of Object.keys(allProperties)) {
    const field = messageType.fields[ord];
    if (!Number.isNaN(Number(propName))) continue; // skip numeric keys
    let value: any;
    if (propName == "metatype") {
      value = metatype;
    } else if (propName == "setProperties") {
      value = []; // never
    } else if (field.repeat) {
      value = [fabricateScalarProp(propName, field)];
    } else {
      value = fabricateScalarProp(propName, field);
    }
    (struct as any)[propName] = value;
    ord += 1;
  }
  return struct as AnyTypeMapping[T];
}

const BENCH_TYPES_NAMES = BENCH_TYPES.map((t) => BenchType[t]);
test.each(BENCH_TYPES_NAMES)(`fabricate(%s)`, (metatype) => {
  fabricate(BenchType[metatype as any] as unknown as BenchType);
});

describe("node graph", () => {
  const graph = new NodeGraph();
  let user1 = fabricate(BenchType.USER, { unset: ["parentPtr"] });
  let clientA = fabricate(BenchType.CLIENT, { set: { parentPtr: toNodeReference(user1) } });
  let clientB = fabricate(BenchType.CLIENT, { set: { parentPtr: toNodeReference(user1) } });
  const user2 = fabricate(BenchType.USER, { unset: ["parentPtr"] });
  const clientC = fabricate(BenchType.CLIENT, { set: { parentPtr: toNodeReference(user2) } });
  const clientD = fabricate(BenchType.CLIENT, { set: { parentPtr: toNodeReference(user2) } });
  const nodes = [user1, clientA, clientB, user2, clientC, clientD];

  // take first refs
  const user1Ref = graph.getRef(user1);
  const user1ClientsRef = graph.getChildrenRef(user1, NodeType.CLIENT);
  test("crud", () => {
    // get
    expect(user1Ref.value).toBeNull();
    expect(user1ClientsRef.value).toEqual([]);

    // create
    graph.extend(user1, clientA, clientB, user2, clientC, clientD);
    nodes.forEach((node) => expect(graph.get({ id: node.id })).toEqual(node));
    expect(graph.getChildren(user1, NodeType.CLIENT)).toEqual([clientA, clientB]);
    expect(graph.getChildren(user2, NodeType.CLIENT)).toEqual([clientC, clientD]);
    expect(user1Ref.value).toEqual(user1);
    expect(user1ClientsRef.value).toEqual([clientA, clientB]);

    // take more refs
    const user2Ref = graph.getRef(user2);
    const user2ClientsRef = graph.getChildrenRef(user2, NodeType.CLIENT);
    expect(user2Ref.value).toEqual(user2);
    expect(user2ClientsRef.value).toEqual([clientC, clientD]);

    // update user
    user1 = { ...user1, name: "user1" } as UserData;
    graph.update(user1);
    expect(graph.get({ id: user1.id })).toEqual(user1);
    expect(user1Ref.value).toEqual(user1);
    // update client
    clientA = { ...clientA, name: "clientA" } as ClientData;
    graph.update(clientA);
    expect(graph.get({ id: clientA.id })).toEqual(clientA);
    expect(graph.getChildren(user1, NodeType.CLIENT)).toEqual([clientA, clientB]);
    expect(user1ClientsRef.value).toEqual([clientA, clientB]);

    // move
    clientB = { ...clientB, parentPtr: toNodeReference(user2) } as ClientData;
    graph.update(clientB);
    expect(graph.getChildren(user1, NodeType.CLIENT)).toEqual([clientA]);
    expect(graph.getChildren(user2, NodeType.CLIENT)).toEqual([clientC, clientD, clientB]);
    expect(user1ClientsRef.value).toEqual([clientA]);
    expect(user2ClientsRef.value).toEqual([clientC, clientD, clientB]);

    // delete
    graph.remove(user1);
    expect(graph.get({ id: user1.id })).toBeNull();
    expect(graph.getChildren(user1, NodeType.CLIENT)).toEqual([]);
    graph.remove(clientC);
    expect(graph.getChildren(user2, NodeType.CLIENT)).toEqual([clientD, clientB]);
    expect(user2ClientsRef.value).toEqual([clientD, clientB]);
  });
});

describe("layered node graph", () => {
  const base = new NodeGraph();
  const overlay = new NodeGraph({ isPartial: true });
  const graph = new LayerNodeGraph([base]);

  let user1 = fabricate(BenchType.USER, { unset: ["parentPtr"] });
  let clientA = fabricate(BenchType.CLIENT, { set: { parentPtr: toNodeReference(user1) } });
  const clientB = fabricate(BenchType.CLIENT, { set: { parentPtr: toNodeReference(user1) } });
  const clientC = fabricate(BenchType.CLIENT, { set: { parentPtr: toNodeReference(user1) } });

  const user1Ref = graph.getRef(user1);
  const user1ClientsRef = graph.getChildrenRef(user1, NodeType.CLIENT);
  const clientARef = graph.getRef(clientA);

  test("crud", async () => {
    // create base
    base.extend(user1, clientA);
    expect(graph.get({ id: user1.id })).toEqual(user1);
    expect(graph.get({ id: clientA.id })).toEqual(clientA);
    expect(graph.getChildren(user1, NodeType.CLIENT)).toEqual([clientA]);
    expect(user1ClientsRef.value).toEqual([clientA]);

    // add overlay
    overlay.extend(clientB);
    graph.addLayer(overlay);
    expect(graph.getChildren(user1, NodeType.CLIENT)).toEqual([clientA, clientB]);
    expect(user1ClientsRef.value).toEqual([clientA, clientB]);

    // update user
    user1 = { ...user1, name: "user1Overlay" } as UserData;
    overlay.update(user1);
    expect(graph.get({ id: user1.id })).toEqual(user1);
    expect(user1Ref.value).toEqual(user1);
    expect(graph.getChildren(user1, NodeType.CLIENT)).toEqual([clientA, clientB]);
    expect(user1ClientsRef.value).toEqual([clientA, clientB]);
    // update client
    clientA = { ...clientA, name: "clientABase" } as ClientData;
    base.update(clientA);
    expect(graph.get({ id: clientA.id })).toEqual(clientA);
    expect(clientARef.value).toEqual(clientA);
    clientA = {
      ...clientA,
      name: "clientAOverlay",
      setProperties: [ClientProperty.setProperties, ClientProperty.name],
    } as ClientData;
    // deviceName in overlay should be ignored because it's not in setProperties
    // (this is a smaller version of the 'higher level' partial update / transaction stuff)
    overlay.update({ ...clientA, deviceName: "ignoreBecauseNotInSetProperties" });
    expect(graph.get({ id: clientA.id })).toEqual(clientA);
    expect(clientARef.value).toEqual(clientA);
    expect(graph.getChildren(user1, NodeType.CLIENT)).toEqual([clientA, clientB]);
    expect(user1ClientsRef.value).toEqual([clientA, clientB]);

    // take more refs
    const clientBRef = graph.getRef(clientB);
    const clientCRef = graph.getRef(clientC);
    expect(clientBRef.value).toEqual(clientB);
    expect(clientCRef.value).toBeNull();

    // add more nodes
    overlay.extend(clientC);
    expect(graph.get({ id: clientC.id })).toEqual(clientC);
    expect(clientCRef.value).toEqual(clientC);
    expect(graph.getChildren(user1, NodeType.CLIENT)).toEqual([clientA, clientB, clientC]);
    expect(user1ClientsRef.value).toEqual([clientA, clientB, clientC]);
  });
});

describe("proxy node graph", () => {
  const baseA = new NodeGraph();
  const baseB = new NodeGraph();
  const graph = new ProxyNodeGraph(null);

  let user1 = fabricate(BenchType.USER, { unset: ["parentPtr"] });
  const clientA = fabricate(BenchType.CLIENT, { set: { parentPtr: toNodeReference(user1) } });
  const user2 = fabricate(BenchType.USER, { unset: ["parentPtr"] });
  const clientB = fabricate(BenchType.CLIENT, { set: { parentPtr: toNodeReference(user2) } });

  const user1Ref = graph.getRef(user1);
  const user1ClientsRef = graph.getChildrenRef(user1, NodeType.CLIENT);
  const user2Ref = graph.getRef(user2);
  const user2ClientsRef = graph.getChildrenRef(user2, NodeType.CLIENT);

  test("crud", async () => {
    expect(user1Ref.value).toBeNull();
    expect(user1ClientsRef.value).toEqual([]);
    expect(user2Ref.value).toBeNull();
    expect(user2ClientsRef.value).toEqual([]);

    // create
    baseA.extend(user1, clientA);
    baseB.extend(user2, clientB);
    graph.graph = baseA;

    // graph = baseA
    expect(graph.get({ id: user1.id })).toEqual(user1);
    expect(graph.getChildren(user1, NodeType.CLIENT)).toEqual([clientA]);
    expect(user1Ref.value).toEqual(user1);
    expect(user1ClientsRef.value).toEqual([clientA]);
    expect(user2Ref.value).toBeNull();
    expect(user2ClientsRef.value).toEqual([]);

    // switch to baseB
    graph.graph = baseB;
    expect(graph.get({ id: user1.id })).toBeNull();
    expect(graph.getChildren(user1, NodeType.CLIENT)).toEqual([]);
    expect(user1Ref.value).toBeNull();
    expect(user1ClientsRef.value).toEqual([]);
    expect(graph.get({ id: user2.id })).toEqual(user2);
    expect(graph.getChildren(user2, NodeType.CLIENT)).toEqual([clientB]);
    expect(user2Ref.value).toEqual(user2);
    expect(user2ClientsRef.value).toEqual([clientB]);

    // update user
    graph.graph = baseA;
    user1 = { ...user1, name: "user1" } as UserData;
    baseA.update(user1);
    expect(graph.get({ id: user1.id })).toEqual(user1);
    expect(user1Ref.value).toEqual(user1);
    expect(graph.getChildren(user1, NodeType.CLIENT)).toEqual([clientA]);
    expect(user1ClientsRef.value).toEqual([clientA]);

    // delete
    graph.graph = baseB;
    baseB.remove(user2);
    expect(graph.get({ id: user2.id })).toBeNull();
    expect(graph.getChildren(user2, NodeType.CLIENT)).toEqual([]);
    expect(user2Ref.value).toBeNull();
    expect(user2ClientsRef.value).toEqual([]);
  });
});

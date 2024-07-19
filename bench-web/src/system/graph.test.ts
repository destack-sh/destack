import {
  OBJECT_TYPE_BY_MESSAGE_TYPE_NAME,
  ObjectType,
  ClientData,
  ClientProperty,
  MESSAGE_TYPE_BY_OBJECT_TYPE,
  NodeType,
  PROPERTY_ENUM_BY_TYPE,
  UserData,
  type AnyNodeData,
  type AnyPropertyType,
  type AnyTypeMapping,
  Timestamp,
  Struct,
} from "@/proto/wire";
import { EMPTY_SCOPE, toNodeReference } from "@/proto/wiring";
import {
  LayerNodeGraph,
  PASSTHROUGH_NODE_FILTER,
  type NodeGraphFilter,
  NodeGraph,
  ProxyNodeGraph,
  type ReadNodeGraph,
  DEFAULT_NODE_FILTER,
} from "@/system/graph";
import { OBJECT_TYPES } from "@/system/lang";
import { ScalarType, type FieldInfo } from "@protobuf-ts/runtime";
import { v4 } from "uuid";
import { describe, expect, test } from "vitest";
import type { Ref } from "vue";

const SCALAR_GENERATORS: Partial<Record<ScalarType, () => any>> = {
  [ScalarType.DOUBLE]: () => Math.random(),
  [ScalarType.FLOAT]: () => Math.random(),
  [ScalarType.INT32]: () => Math.floor(Math.random() * 0x7fffffff),
  [ScalarType.INT64]: () => BigInt(Math.floor(Math.random() * 0x7fffffff)),
  [ScalarType.UINT32]: () => Math.floor(Math.random() * 0x7fffffff),
  [ScalarType.UINT64]: () => Math.floor(Math.random() * 0x7fffffff),
  [ScalarType.SINT32]: () => Math.floor(Math.random() * 0x7fffffff),
  [ScalarType.SINT64]: () => Math.floor(Math.random() * 0x7fffffff),
  [ScalarType.BOOL]: () => Math.random() > 0.5,
  [ScalarType.STRING]: () => v4(),
  [ScalarType.BYTES]: () => v4(),
};

const MESSAGE_TYPE_GENERATORS: Record<string, () => any> = {
  "google.protobuf.Struct": () => Struct.fromJson({}),
  "google.protobuf.Timestamp": () => Timestamp.now(),
  "google.protobuf.Duration": () => Math.random(),
  "symbolx.bench.NodeReferenceData": () => ({ metatype: ObjectType.NODE_REFERENCE, id: v4(), type: NodeType.USER }),
};

const PROP_NAME_GENERATORS: Record<string, () => any> = {
  valuePacked: () => {},
  oldNodePartial: () => {},
  newNodePartial: () => {},
};

const MEMBERS_BY_ENUM: Record<string, number[]> = {};

export function fabricate<T extends ObjectType>(
  metatype: T,
  options?: { path?: ObjectType[]; unset?: (keyof AnyTypeMapping[T])[]; set?: Partial<AnyTypeMapping[T]> },
): AnyTypeMapping[T] {
  const allProperties: AnyPropertyType | undefined = PROPERTY_ENUM_BY_TYPE[metatype];
  if (allProperties == null) throw new Error(`no properties for ${ObjectType[metatype]}`);
  const messageType = MESSAGE_TYPE_BY_OBJECT_TYPE[metatype]!;

  function fabricateScalarProp(propName: string, field: FieldInfo): any {
    let value: any;
    if (options?.set != null && (options.set as any)[propName] !== undefined) {
      value = (options.set as any)[propName];
    } else if (options?.unset?.includes(propName as any)) {
      value = undefined;
    } else if (field.name.endsWith("_id") || field.name.endsWith("_ck")) {
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
    } else if (field.kind == "message" && OBJECT_TYPE_BY_MESSAGE_TYPE_NAME[field.T().typeName]) {
      const benchType = OBJECT_TYPE_BY_MESSAGE_TYPE_NAME[field.T().typeName]!;
      const path = (options?.path ?? []).concat(metatype);
      value = fabricate(benchType, { path });
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
    // skip recursive fields
    const fieldObjectType = field.kind == "message" ? OBJECT_TYPE_BY_MESSAGE_TYPE_NAME[field.T().typeName] : null;
    if (fieldObjectType && options?.path?.includes(fieldObjectType)) {
      // skip recursive fields
      value = field.repeat ? [] : null;
    } else if (propName == "metatype") {
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

const OBJECT_TYPES_NAMES = OBJECT_TYPES.map((t) => ObjectType[t]);
test.each(OBJECT_TYPES_NAMES)(`fabricate(%s)`, (metatype) => {
  fabricate(ObjectType[metatype as any] as unknown as ObjectType);
});

describe("node graph", () => {
  const graph = new NodeGraph({ scope: EMPTY_SCOPE, nodeTypes: [NodeType.USER, NodeType.CLIENT] });
  let user1 = fabricate(ObjectType.USER, { unset: ["parentPtr"], set: { id: "user1" } });
  let clientA = fabricate(ObjectType.CLIENT, { set: { parentPtr: toNodeReference(user1), id: "clientA" } });
  let clientB = fabricate(ObjectType.CLIENT, { set: { parentPtr: toNodeReference(user1), id: "clientB" } });
  const user2 = fabricate(ObjectType.USER, { unset: ["parentPtr"], set: { id: "user2" } });
  const clientC = fabricate(ObjectType.CLIENT, { set: { parentPtr: toNodeReference(user2), id: "clientC" } });
  const clientD = fabricate(ObjectType.CLIENT, { set: { parentPtr: toNodeReference(user2), id: "clientD" } });
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
    expect(graph.getChildren(user2, NodeType.CLIENT)).toEqual([clientB, clientC, clientD]);
    expect(user1ClientsRef.value).toEqual([clientA]);
    expect(user2ClientsRef.value).toEqual([clientB, clientC, clientD]);

    // delete
    graph.remove(user1);
    expect(graph.get({ id: user1.id })).toBeNull();
    expect(graph.getChildren(user1, NodeType.CLIENT)).toEqual([]);
    graph.remove(clientC);
    expect(graph.getChildren(user2, NodeType.CLIENT)).toEqual([clientB, clientD]);
    expect(user2ClientsRef.value).toEqual([clientB, clientD]);
  });
});

describe("layered node graph", () => {
  const base = new NodeGraph({ scope: EMPTY_SCOPE, nodeTypes: [NodeType.USER, NodeType.CLIENT] });
  const overlay = new NodeGraph({ scope: base.scope, nodeTypes: base.nodeTypes, isOverlayOf: base });
  const graph = new LayerNodeGraph({ layers: [base], filter: PASSTHROUGH_NODE_FILTER });

  let user1 = fabricate(ObjectType.USER, { unset: ["parentPtr"], set: { id: "user1" } });
  let clientA = fabricate(ObjectType.CLIENT, { set: { parentPtr: toNodeReference(user1), id: "clientA" } });
  const clientB = fabricate(ObjectType.CLIENT, { set: { parentPtr: toNodeReference(user1), id: "clientB" } });
  const clientC = fabricate(ObjectType.CLIENT, { set: { parentPtr: toNodeReference(user1), id: "clientC" } });
  const clientD = fabricate(ObjectType.CLIENT, { set: { parentPtr: toNodeReference(user1), id: "clientD" } });
  const user2 = fabricate(ObjectType.USER, { unset: ["parentPtr"], set: { id: "user2" } });

  const user1Ref = graph.getRef(user1);
  const user1ClientsRef = graph.getChildrenRef(user1, NodeType.CLIENT);
  const clientARef = graph.getRef({ id: clientA.id });

  test("crud", () => {
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
    // NOTE: deviceName in overlay should be ignored because it's not in setProperties.
    //  This shouldn't really happen, but it's good to have this invariant.
    overlay.update({ ...clientA, deviceName: "ignoreBecauseNotInSetProperties" });
    expect(graph.get({ id: clientA.id })).toEqual(clientA);
    expect(clientARef.value).toEqual(clientA);
    expect(graph.getChildren(user1, NodeType.CLIENT)).toEqual([clientA, clientB]);
    expect(user1ClientsRef.value).toEqual([clientA, clientB]);

    // take more refs
    const clientBRef = graph.getRef({ id: clientB.id });
    const clientCRef = graph.getRef({ id: clientC.id });
    expect(clientBRef.value).toEqual(clientB);
    expect(clientCRef.value).toBeNull();

    // add clientC
    overlay.extend(clientC);
    expect(graph.get({ id: clientC.id })).toEqual(clientC);
    expect(clientCRef.value).toEqual(clientC);
    expect(graph.getChildren(user1, NodeType.CLIENT)).toEqual([clientA, clientB, clientC]);
    expect(user1ClientsRef.value).toEqual([clientA, clientB, clientC]);

    // move clientA to user 2
    const user2ClientsRef = graph.getChildrenRef(user2, NodeType.CLIENT);
    base.extend(user2);
    clientA = { ...clientA, parentPtr: toNodeReference(user2) } as ClientData;
    overlay.update(clientA);
    expect(graph.getChildren(user1, NodeType.CLIENT)).toEqual([clientB, clientC]);
    expect(graph.getChildren(user2, NodeType.CLIENT)).toEqual([clientA]);
    expect(user1ClientsRef.value).toEqual([clientB, clientC]);
    expect(user2ClientsRef.value).toEqual([clientA]);

    // remove clientB in overlay
    overlay.remove(clientB);
    expect(graph.getChildren(user1, NodeType.CLIENT)).toEqual([clientC]);
    expect(user1ClientsRef.value).toEqual([clientC]);
  });
});

describe("proxy node graph", () => {
  const baseA = new NodeGraph({ scope: EMPTY_SCOPE, nodeTypes: [NodeType.USER, NodeType.CLIENT] });
  const baseB = new NodeGraph({ scope: EMPTY_SCOPE, nodeTypes: [NodeType.USER, NodeType.CLIENT] });
  const graph = new ProxyNodeGraph({ filter: PASSTHROUGH_NODE_FILTER });

  let user1 = fabricate(ObjectType.USER, { unset: ["parentPtr"], set: { id: "user1" } });
  const clientA = fabricate(ObjectType.CLIENT, { set: { parentPtr: toNodeReference(user1), id: "clientA" } });
  const user2 = fabricate(ObjectType.USER, { unset: ["parentPtr"], set: { id: "user2" } });
  const clientB = fabricate(ObjectType.CLIENT, { set: { parentPtr: toNodeReference(user2), id: "clientB" } });

  const user1Ref = graph.getRef(user1);
  const user1ClientsRef = graph.getChildrenRef(user1, NodeType.CLIENT);
  const user2Ref = graph.getRef(user2);
  const user2ClientsRef = graph.getChildrenRef(user2, NodeType.CLIENT);

  test("crud", () => {
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

function testFilteredGraph(base: NodeGraph, graph: ReadNodeGraph & { filter: Ref<NodeGraphFilter> }) {
  let package1 = fabricate(ObjectType.PACKAGE, { unset: ["parentPtr", "archivedAt", "deletedAt"] });
  const space11 = fabricate(ObjectType.SPACE, {
    unset: ["archivedAt", "deletedAt"],
    set: { parentPtr: toNodeReference(package1), id: "space11", orderKey: "a0" },
  });
  let view111 = fabricate(ObjectType.VIEW, {
    unset: ["archivedAt", "deletedAt"],
    set: { parentPtr: toNodeReference(space11), id: "view111", orderKey: "a0" },
  });
  const view112 = fabricate(ObjectType.VIEW, {
    unset: ["archivedAt", "deletedAt"],
    set: { parentPtr: toNodeReference(space11), id: "view112", orderKey: "a1" },
  });
  let space12 = fabricate(ObjectType.SPACE, {
    unset: ["archivedAt", "deletedAt"],
    set: { parentPtr: toNodeReference(package1), id: "space12", orderKey: "a1" },
  });
  const view121 = fabricate(ObjectType.VIEW, {
    unset: ["archivedAt", "deletedAt"],
    set: { parentPtr: toNodeReference(space12), id: "view121", orderKey: "a0" },
  });

  const package1Ref = graph.getRef(package1);
  const package1SpacesRef = graph.getChildrenRef(package1, NodeType.SPACE);
  const space11Ref = graph.getRef({ id: space11.id });
  const space11ViewsRef = graph.getChildrenRef(space11, NodeType.VIEW);
  const view111Ref = graph.getRef({ id: view111.id });
  const view112Ref = graph.getRef({ id: view112.id });
  const space12Ref = graph.getRef({ id: space12.id });
  const space12ViewsRef = graph.getChildrenRef(space12, NodeType.VIEW);
  const view121Ref = graph.getRef({ id: view121.id });

  function hide<T extends AnyNodeData>(obj: T): T {
    return { ...obj, archivedAt: null, deletedAt: new Date().toISOString() } as T;
  }
  function show<T extends AnyNodeData>(obj: T): T {
    return { ...obj, archivedAt: null, deletedAt: null } as T;
  }

  test("crud", () => {
    view111 = hide(view111);
    space12 = hide(space12);

    // create
    base.extend(package1, space11, view111, view112, space12, view121);

    // no filter -> get all
    graph.filter.value = PASSTHROUGH_NODE_FILTER;
    expect(graph.get({ id: view111.id })).toEqual(view111);
    expect(view111Ref.value).toEqual(view111);
    expect(graph.get({ id: space12.id })).toEqual(space12);
    expect(space12Ref.value).toEqual(space12);
    expect(graph.get({ id: view121.id })).toEqual(view121);
    expect(view121Ref.value).toEqual(view121);
    expect(graph.getChildren(space11, NodeType.VIEW)).toEqual([view111, view112]);
    expect(space11ViewsRef.value).toEqual([view111, view112]);
    expect(graph.getChildren(space12, NodeType.VIEW)).toEqual([view121]);
    expect(space12ViewsRef.value).toEqual([view121]);

    // enable filter -> get unfiltered
    graph.filter.value = DEFAULT_NODE_FILTER;
    expect(graph.get({ id: view111.id })).toBeNull();
    expect(view111Ref.value).toBeNull();
    expect(graph.get({ id: space12.id })).toBeNull();
    expect(space12Ref.value).toBeNull();
    expect(graph.get({ id: view121.id })).toBeNull();
    expect(view121Ref.value).toBeNull();
    expect(graph.getChildren(space11, NodeType.VIEW)).toEqual([view112]);
    expect(space11ViewsRef.value).toEqual([view112]);
    expect(graph.getChildren(space12, NodeType.VIEW)).toEqual([]);
    expect(space12ViewsRef.value).toEqual([]);

    // show & re-hide leaf
    view111 = show(view111);
    base.update(view111);
    expect(graph.get({ id: view111.id })).toEqual(view111);
    expect(view111Ref.value).toEqual(view111);
    view111 = hide(view111);
    base.update(view111);
    expect(graph.get({ id: view111.id })).toBeNull();
    expect(view111Ref.value).toBeNull();

    // show & re-hide parent
    space12 = show(space12);
    base.update(space12);
    expect(graph.get({ id: space12.id })).toEqual(space12);
    expect(space12Ref.value).toEqual(space12);
    expect(graph.getChildren(space12, NodeType.VIEW)).toEqual([view121]);
    expect(space12ViewsRef.value).toEqual([view121]);
    expect(graph.get({ id: view121.id })).toEqual(view121);
    expect(view121Ref.value).toEqual(view121);
    space12 = hide(space12);
    base.update(space12);
    expect(graph.get({ id: space12.id })).toBeNull();
    expect(space12Ref.value).toBeNull();
    expect(graph.getChildren(space12, NodeType.VIEW)).toEqual([]);
    expect(space12ViewsRef.value).toEqual([]);
    expect(graph.get({ id: view121.id })).toBeNull();
    expect(view121Ref.value).toBeNull();

    // hide root
    package1 = hide(package1);
    base.update(package1);
    expect(graph.get({ id: package1.id })).toBeNull();
    expect(package1Ref.value).toBeNull();
    expect(graph.getChildren(package1, NodeType.SPACE)).toEqual([]);
    expect(package1SpacesRef.value).toEqual([]);
    expect(graph.get({ id: space11.id })).toBeNull();
    expect(space11Ref.value).toBeNull();
  });
}

describe("filtered proxy graph", () => {
  const base = new NodeGraph({ scope: EMPTY_SCOPE, nodeTypes: [NodeType.PACKAGE, NodeType.SPACE, NodeType.VIEW] });
  const graph = new ProxyNodeGraph({ graph: base, filter: PASSTHROUGH_NODE_FILTER });
  testFilteredGraph(base, graph);
});
describe("filtered layered graph", () => {
  const base = new NodeGraph({ scope: EMPTY_SCOPE, nodeTypes: [NodeType.PACKAGE, NodeType.SPACE, NodeType.VIEW] });
  const graph = new LayerNodeGraph({ layers: [base], filter: PASSTHROUGH_NODE_FILTER });
  testFilteredGraph(base, graph);
});

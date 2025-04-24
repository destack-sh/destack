import { OBJECT_TYPES } from "@/language/core/const";
import {
  DEFAULT_NODE_FILTER,
  LayerNodeGraph,
  mergeNode,
  NodeGraph,
  PASSTHROUGH_NODE_FILTER,
  ProxyNodeGraph,
  type NodeGraphFilter,
  type ReadNodeGraph,
} from "@/language/core/graph";
import { makeNode } from "@/language/core/node";
import {
  ClientData,
  ClientProperty,
  FlowData,
  FlowProperty,
  MESSAGE_TYPE_BY_OBJECT_TYPE,
  NodeType,
  OBJECT_TYPE_BY_MESSAGE_TYPE_NAME,
  ObjectType,
  PageData,
  PageProperty,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_TYPE,
  Struct,
  Timestamp,
  UserData,
  UserProperty,
  Value,
  ViewProperty,
  type AnyNodeData,
  type AnyPropertyType,
  type AnyTypeMapping,
  type PropertyInfo,
} from "@/proto/wire";
import { Duration } from "@/proto/wire/google/protobuf/duration";
import { EMPTY_SCOPE, toNodeRef } from "@/proto/wiring";
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
  "google.protobuf.Value": () => Value.fromJson(null),
  "google.protobuf.Timestamp": () => Timestamp.now(),
  "google.protobuf.Duration": (): Duration => {
    const seconds = BigInt(Math.floor(Math.random() * 60));
    const nanos = Math.floor(Math.random() * 1e9);
    return { seconds, nanos };
  },
  "symbolx.bench.NodeReferenceData": () => ({ metatype: ObjectType.NODE_REFERENCE, id: v4(), nodeType: NodeType.USER }),
};

const PROP_NAME_GENERATORS: Record<string, () => any> = {
  valuePacked: () => {},
  nodeData: () => {},
};

const MEMBERS_BY_ENUM: Record<string, number[]> = {};

export function fabricate<T extends ObjectType>(
  metatype: T,
  options?: { path?: ObjectType[]; unset?: (keyof AnyTypeMapping[T])[]; set?: Partial<AnyTypeMapping[T]> },
): AnyTypeMapping[T] {
  const allProperties: AnyPropertyType | undefined = PROPERTY_ENUM_BY_TYPE[metatype];
  if (allProperties == null) throw new Error(`no properties for ${ObjectType[metatype]}`);
  const propertyInfos = PROPERTY_INFOS_BY_TYPE[metatype];
  const messageType = MESSAGE_TYPE_BY_OBJECT_TYPE[metatype]!;

  function fabricateScalarProp(propName: string, field: FieldInfo | undefined, prop: PropertyInfo): any {
    let value: any;
    if (options?.set != null && (options.set as any)[propName] !== undefined) {
      value = (options.set as any)[propName];
    } else if (options?.unset?.includes(propName as any)) {
      value = undefined;
    } else if (prop.name.endsWith("_id") || prop.name.endsWith("_ck")) {
      value = v4();
    } else if (field?.kind == "scalar" && SCALAR_GENERATORS[field.T] != null) {
      value = SCALAR_GENERATORS[field.T]!();
    } else if (field?.kind == "enum") {
      const [typeName, enu] = field.T();
      if (MEMBERS_BY_ENUM[typeName] == null) {
        MEMBERS_BY_ENUM[typeName] = Object.keys(enu)
          .map((n) => Number(n))
          .filter((n) => !isNaN(n));
      }
      const members = MEMBERS_BY_ENUM[typeName];
      value = members[Math.floor(Math.random() * members.length)];
    } else if (field?.kind == "message" && OBJECT_TYPE_BY_MESSAGE_TYPE_NAME[field.T().typeName]) {
      const benchType = OBJECT_TYPE_BY_MESSAGE_TYPE_NAME[field.T().typeName]!;
      const path = (options?.path ?? []).concat(metatype);
      value = fabricate(benchType, { path });
    } else if (field?.kind == "message" && MESSAGE_TYPE_GENERATORS[field.T().typeName]) {
      value = MESSAGE_TYPE_GENERATORS[field.T().typeName]!();
    } else if (PROP_NAME_GENERATORS[propName]) {
      value = PROP_NAME_GENERATORS[propName]!();
    } else {
      throw new Error(
        `no generator for field ${messageType.typeName}.${propName} [prop=${prop.id}, kind=${field?.kind}]`,
      );
    }
    return value;
  }

  const struct = {};
  for (const propName of Object.keys(allProperties)) {
    if (!Number.isNaN(Number(propName))) continue; // skip numeric keys
    const propId = allProperties[propName as any] as unknown as number;
    const prop = propertyInfos[propId];
    const field = messageType.fields.find((f) => f.no == propId);

    let value: any;
    if (prop.referenceStruct && options?.path?.includes(prop.referenceStruct as unknown as ObjectType)) {
      // skip recursive fields
      value = prop.isList ? [] : undefined;
    } else if (propName == "metatype") {
      value = metatype;
    } else if (prop.isList) {
      value = [fabricateScalarProp(propName, field, prop)];
    } else {
      value = fabricateScalarProp(propName, field, prop);
    }
    (struct as any)[propName] = value;
  }
  return struct as AnyTypeMapping[T];
}

const OBJECT_TYPES_NAMES = OBJECT_TYPES.map((t) => ObjectType[t]);
test.each(OBJECT_TYPES_NAMES)(`fabricate(%s)`, (metatype) => {
  fabricate(ObjectType[metatype as any] as unknown as ObjectType);
});

describe("merge nodes", () => {
  const bench = makeNode({ metatype: NodeType.BENCH, name: "test", slug: "test" });
  const pkg = makeNode({ metatype: NodeType.PACKAGE, benchPtr: toNodeRef(bench) });
  const base1 = makeNode({
    metatype: NodeType.FLOW,
    packagePtr: toNodeRef(pkg),
    name: "Block1",
  });

  test("merge", () => {
    // simple merge without changes
    const overlay1 = structuredClone(base1);
    const merged1 = mergeNode(base1, overlay1);
    expect(merged1).toEqual(base1);

    // changes that are not in setPaths should be ignored
    const overlay2 = structuredClone(base1);
    overlay2.name = "Overlay2";
    const merged2 = mergeNode(base1, overlay2);
    expect(merged2).toEqual(base1);

    // changes in setPaths should be applied (and irrelevant setPaths should be ignored)
    const overlay3 = structuredClone(base1);
    overlay3.name = "Overlay3";
    (overlay3 as any).setPaths = [[FlowProperty.name.toString()]];
    const merged3 = mergeNode(base1, overlay3) as FlowData;
    expect(merged3.name).toEqual("Overlay3");
  });
});

describe("node graph", () => {
  const graph = new NodeGraph({ scope: EMPTY_SCOPE, nodeTypes: new Set([NodeType.USER, NodeType.CLIENT]) });
  let user1 = fabricate(ObjectType.USER, { unset: ["parentPtr"], set: { id: "user1" } });
  let clientA = fabricate(ObjectType.CLIENT, { set: { parentPtr: toNodeRef(user1), id: "clientA" } });
  let clientB = fabricate(ObjectType.CLIENT, { set: { parentPtr: toNodeRef(user1), id: "clientB" } });
  const user2 = fabricate(ObjectType.USER, { unset: ["parentPtr"], set: { id: "user2" } });
  const clientC = fabricate(ObjectType.CLIENT, { set: { parentPtr: toNodeRef(user2), id: "clientC" } });
  const clientD = fabricate(ObjectType.CLIENT, { set: { parentPtr: toNodeRef(user2), id: "clientD" } });
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
    clientB = { ...clientB, parentPtr: toNodeRef(user2) } as ClientData;
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
  const base = new NodeGraph({ scope: EMPTY_SCOPE, nodeTypes: new Set([NodeType.USER, NodeType.CLIENT]) });
  const overlay = new NodeGraph({ scope: base.scope, nodeTypes: base.nodeTypes, isOverlayOf: base });
  const composite = new LayerNodeGraph({ layers: [base], filter: PASSTHROUGH_NODE_FILTER });

  let user1 = fabricate(ObjectType.USER, { unset: ["parentPtr"], set: { id: "user1" } });
  let clientA = fabricate(ObjectType.CLIENT, { set: { parentPtr: toNodeRef(user1), id: "clientA" } });
  const clientB = fabricate(ObjectType.CLIENT, { set: { parentPtr: toNodeRef(user1), id: "clientB" } });
  const clientC = fabricate(ObjectType.CLIENT, { set: { parentPtr: toNodeRef(user1), id: "clientC" } });
  const clientD = fabricate(ObjectType.CLIENT, { set: { parentPtr: toNodeRef(user1), id: "clientD" } });
  const user2 = fabricate(ObjectType.USER, { unset: ["parentPtr"], set: { id: "user2" } });

  const user1Ref = composite.getRef(user1);
  const user1ClientsRef = composite.getChildrenRef(user1, NodeType.CLIENT);
  const clientARef = composite.getRef({ id: clientA.id });

  test("crud", () => {
    // create base
    base.extend(user1, clientA);
    expect(composite.get({ id: user1.id })).toEqual(user1);
    expect(composite.get({ id: clientA.id })).toEqual(clientA);
    expect(composite.getChildren(user1, NodeType.CLIENT)).toEqual([clientA]);
    expect(user1ClientsRef.value).toEqual([clientA]);

    // add overlay
    overlay.extend(clientB);
    composite.addLayer(overlay);
    expect(composite.getChildren(user1, NodeType.CLIENT)).toEqual([clientA, clientB]);
    expect(user1ClientsRef.value).toEqual([clientA, clientB]);

    // update user
    user1 = { ...user1, name: "user1Overlay", setPaths: [[UserProperty.name.toString()]] } as UserData;
    overlay.update(user1);
    expect(composite.get({ id: user1.id })).toEqual(user1);
    expect(user1Ref.value).toEqual(user1);
    expect(composite.getChildren(user1, NodeType.CLIENT)).toEqual([clientA, clientB]);
    expect(user1ClientsRef.value).toEqual([clientA, clientB]);
    // update client
    clientA = { ...clientA, title: "clientABase" } as ClientData;
    base.update(clientA);
    expect(composite.get({ id: clientA.id })).toEqual(clientA);
    expect(clientARef.value).toEqual(clientA);
    clientA = {
      ...clientA,
      name: "clientAOverlay",
      setPaths: [[ClientProperty.name.toString()]],
    } as ClientData;
    // NOTE: deviceName in overlay should be ignored because it's not in setPaths.
    //  This shouldn't really happen, but it's good to have this invariant.
    overlay.update({ ...clientA, deviceName: "ignoreBecauseNotInSetPaths" });
    expect(composite.get({ id: clientA.id })).toEqual(clientA);
    expect(clientARef.value).toEqual(clientA);
    expect(composite.getChildren(user1, NodeType.CLIENT)).toEqual([clientA, clientB]);
    expect(user1ClientsRef.value).toEqual([clientA, clientB]);

    // take more refs
    const clientBRef = composite.getRef({ id: clientB.id });
    const clientCRef = composite.getRef({ id: clientC.id });
    expect(clientBRef.value).toEqual(clientB);
    expect(clientCRef.value).toBeNull();

    // add clientC
    overlay.extend(clientC);
    expect(composite.get({ id: clientC.id })).toEqual(clientC);
    expect(clientCRef.value).toEqual(clientC);
    expect(composite.getChildren(user1, NodeType.CLIENT)).toEqual([clientA, clientB, clientC]);
    expect(user1ClientsRef.value).toEqual([clientA, clientB, clientC]);

    // move clientA to user 2
    const user2ClientsRef = composite.getChildrenRef(user2, NodeType.CLIENT);
    base.extend(user2);
    clientA = { ...clientA, parentPtr: toNodeRef(user2) } as ClientData;
    overlay.update(clientA);
    expect(composite.getChildren(user1, NodeType.CLIENT)).toEqual([clientB, clientC]);
    expect(composite.getChildren(user2, NodeType.CLIENT)).toEqual([clientA]);
    expect(user1ClientsRef.value).toEqual([clientB, clientC]);
    expect(user2ClientsRef.value).toEqual([clientA]);

    // remove clientB in overlay
    overlay.remove(clientB);
    expect(composite.getChildren(user1, NodeType.CLIENT)).toEqual([clientC]);
    expect(user1ClientsRef.value).toEqual([clientC]);
  });
});

describe("proxy node graph", () => {
  const baseA = new NodeGraph({ scope: EMPTY_SCOPE, nodeTypes: new Set([NodeType.USER, NodeType.CLIENT]) });
  const baseB = new NodeGraph({ scope: EMPTY_SCOPE, nodeTypes: new Set([NodeType.USER, NodeType.CLIENT]) });
  const proxy = new ProxyNodeGraph({ filter: PASSTHROUGH_NODE_FILTER });

  let user1 = fabricate(ObjectType.USER, { unset: ["parentPtr"], set: { id: "user1" } });
  const clientA = fabricate(ObjectType.CLIENT, { set: { parentPtr: toNodeRef(user1), id: "clientA" } });
  const user2 = fabricate(ObjectType.USER, { unset: ["parentPtr"], set: { id: "user2" } });
  const clientB = fabricate(ObjectType.CLIENT, { set: { parentPtr: toNodeRef(user2), id: "clientB" } });

  const user1Ref = proxy.getRef(user1);
  const user1ClientsRef = proxy.getChildrenRef(user1, NodeType.CLIENT);
  const user2Ref = proxy.getRef(user2);
  const user2ClientsRef = proxy.getChildrenRef(user2, NodeType.CLIENT);

  test("crud", () => {
    expect(user1Ref.value).toBeNull();
    expect(user1ClientsRef.value).toEqual([]);
    expect(user2Ref.value).toBeNull();
    expect(user2ClientsRef.value).toEqual([]);

    // create
    baseA.extend(user1, clientA);
    baseB.extend(user2, clientB);
    proxy.graph = baseA;

    // graph = baseA
    expect(proxy.get({ id: user1.id })).toEqual(user1);
    expect(proxy.getChildren(user1, NodeType.CLIENT)).toEqual([clientA]);
    expect(user1Ref.value).toEqual(user1);
    expect(user1ClientsRef.value).toEqual([clientA]);
    expect(user2Ref.value).toBeNull();
    expect(user2ClientsRef.value).toEqual([]);

    // switch to baseB
    proxy.graph = baseB;
    expect(proxy.get({ id: user1.id })).toBeNull();
    expect(proxy.getChildren(user1, NodeType.CLIENT)).toEqual([]);
    expect(user1Ref.value).toBeNull();
    expect(user1ClientsRef.value).toEqual([]);
    expect(proxy.get({ id: user2.id })).toEqual(user2);
    expect(proxy.getChildren(user2, NodeType.CLIENT)).toEqual([clientB]);
    expect(user2Ref.value).toEqual(user2);
    expect(user2ClientsRef.value).toEqual([clientB]);

    // update user
    proxy.graph = baseA;
    user1 = { ...user1, name: "user1" } as UserData;
    baseA.update(user1);
    expect(proxy.get({ id: user1.id })).toEqual(user1);
    expect(user1Ref.value).toEqual(user1);
    expect(proxy.getChildren(user1, NodeType.CLIENT)).toEqual([clientA]);
    expect(user1ClientsRef.value).toEqual([clientA]);

    // delete
    proxy.graph = baseB;
    baseB.remove(user2);
    expect(proxy.get({ id: user2.id })).toBeNull();
    expect(proxy.getChildren(user2, NodeType.CLIENT)).toEqual([]);
    expect(user2Ref.value).toBeNull();
    expect(user2ClientsRef.value).toEqual([]);
  });
});

function markDeleted<T extends AnyNodeData>(obj: T): T {
  return { ...obj, deletedAt: new Date().toISOString() } as T;
}
function markUndeleted<T extends AnyNodeData>(obj: T): T {
  return { ...obj, deletedAt: null } as T;
}

function _baseTestFilteredGraph(base: NodeGraph, composite: ReadNodeGraph & { filter: Ref<NodeGraphFilter> }) {
  let package1 = fabricate(ObjectType.PACKAGE, {
    unset: ["parentPtr", "deletedAt", "archivedAt"],
    set: { id: "package1" },
  });
  const space11 = fabricate(ObjectType.SPACE, {
    unset: ["archivedAt", "deletedAt"],
    set: { parentPtr: toNodeRef(package1), id: "space11", orderKey: "a0" },
  });
  let view111 = fabricate(ObjectType.VIEW, {
    unset: ["archivedAt", "deletedAt"],
    set: { parentPtr: toNodeRef(space11), id: "view111", orderKey: "a0" },
  });
  const view112 = fabricate(ObjectType.VIEW, {
    unset: ["archivedAt", "deletedAt"],
    set: { parentPtr: toNodeRef(space11), id: "view112", orderKey: "a1" },
  });
  let space12 = fabricate(ObjectType.SPACE, {
    unset: ["archivedAt", "deletedAt"],
    set: { parentPtr: toNodeRef(package1), id: "space12", orderKey: "a1" },
  });
  const view121 = fabricate(ObjectType.VIEW, {
    unset: ["archivedAt", "deletedAt"],
    set: { parentPtr: toNodeRef(space12), id: "view121", orderKey: "a0" },
  });

  const package1Ref = composite.getRef(package1);
  const package1SpacesRef = composite.getChildrenRef(package1, NodeType.SPACE);
  const space11Ref = composite.getRef({ id: space11.id });
  const space11ViewsRef = composite.getChildrenRef(space11, NodeType.VIEW);
  const view111Ref = composite.getRef({ id: view111.id });
  const view112Ref = composite.getRef({ id: view112.id });
  const space12Ref = composite.getRef({ id: space12.id });
  const space12ViewsRef = composite.getChildrenRef(space12, NodeType.VIEW);
  const view121Ref = composite.getRef({ id: view121.id });

  test("crud", () => {
    view111 = markDeleted(view111);
    space12 = markDeleted(space12);

    // create
    base.extend(package1, space11, view111, view112, space12, view121);

    // no filter -> get all
    composite.filter.value = PASSTHROUGH_NODE_FILTER;
    expect(composite.get({ id: view111.id })).toEqual(view111);
    expect(view111Ref.value).toEqual(view111);
    expect(composite.get({ id: space12.id })).toEqual(space12);
    expect(space12Ref.value).toEqual(space12);
    expect(composite.get({ id: view121.id })).toEqual(view121);
    expect(view121Ref.value).toEqual(view121);
    expect(composite.getChildren(space11, NodeType.VIEW)).toEqual([view111, view112]);
    expect(space11ViewsRef.value).toEqual([view111, view112]);
    expect(composite.getChildren(space12, NodeType.VIEW)).toEqual([view121]);
    expect(space12ViewsRef.value).toEqual([view121]);

    // enable filter -> get filtered
    composite.filter.value = DEFAULT_NODE_FILTER;
    expect(composite.get({ id: view111.id })).toBeNull();
    expect(view111Ref.value).toBeNull();
    expect(composite.get({ id: space12.id })).toBeNull();
    expect(space12Ref.value).toBeNull();
    expect(composite.get({ id: view121.id })).toBeNull();
    expect(view121Ref.value).toBeNull();
    expect(composite.getChildren(space11, NodeType.VIEW)).toEqual([view112]);
    expect(space11ViewsRef.value).toEqual([view112]);
    expect(composite.getChildren(space12, NodeType.VIEW)).toEqual([]);
    expect(space12ViewsRef.value).toEqual([]);

    // show & re-hide leaf
    view111 = markUndeleted(view111);
    base.update(view111);
    expect(composite.get({ id: view111.id })).toEqual(view111);
    expect(view111Ref.value).toEqual(view111);
    view111 = markDeleted(view111);
    base.update(view111);
    expect(composite.get({ id: view111.id })).toBeNull();
    expect(view111Ref.value).toBeNull();

    // show & re-hide parent
    space12 = markUndeleted(space12);
    base.update(space12);
    expect(composite.get({ id: space12.id })).toEqual(space12);
    expect(space12Ref.value).toEqual(space12);
    expect(composite.getChildren(space12, NodeType.VIEW)).toEqual([view121]);
    expect(space12ViewsRef.value).toEqual([view121]);
    expect(composite.get({ id: view121.id })).toEqual(view121);
    expect(view121Ref.value).toEqual(view121);
    space12 = markDeleted(space12);
    base.update(space12);
    expect(composite.get({ id: space12.id })).toBeNull();
    expect(space12Ref.value).toBeNull();
    expect(composite.getChildren(space12, NodeType.VIEW)).toEqual([]);
    expect(space12ViewsRef.value).toEqual([]);
    expect(composite.get({ id: view121.id })).toBeNull();
    expect(view121Ref.value).toBeNull();

    // hide & show root
    package1 = markDeleted(package1);
    base.update(package1);
    expect(composite.get({ id: package1.id })).toBeNull();
    expect(package1Ref.value).toBeNull();
    expect(composite.getChildren(package1, NodeType.SPACE)).toEqual([]);
    expect(package1SpacesRef.value).toEqual([]);
    expect(composite.get({ id: space11.id })).toBeNull();
    expect(space11Ref.value).toBeNull();
    package1 = markUndeleted(package1);
    base.update(package1);
    expect(composite.get({ id: package1.id })).toEqual(package1);
    expect(package1Ref.value).toEqual(package1);
    expect(composite.getChildren(package1, NodeType.SPACE)).toEqual([space11]);
    expect(package1SpacesRef.value).toEqual([space11]);
  });
}

describe("filtered proxy graph", () => {
  const base = new NodeGraph({
    scope: EMPTY_SCOPE,
    nodeTypes: new Set([NodeType.PACKAGE, NodeType.SPACE, NodeType.VIEW]),
  });
  const composite = new ProxyNodeGraph({ graph: base, filter: PASSTHROUGH_NODE_FILTER });
  _baseTestFilteredGraph(base, composite);
});

describe("filtered layered graph", () => {
  const base = new NodeGraph({
    scope: EMPTY_SCOPE,
    nodeTypes: new Set([NodeType.PACKAGE, NodeType.SPACE, NodeType.VIEW]),
  });
  const overlay = new NodeGraph({ scope: base.scope, nodeTypes: base.nodeTypes, isOverlayOf: base });
  const composite = new LayerNodeGraph({ layers: [base, overlay], filter: PASSTHROUGH_NODE_FILTER });
  _baseTestFilteredGraph(base, composite);

  test("filtered layered crud", () => {
    const package9 = fabricate(ObjectType.PACKAGE, { unset: ["parentPtr", "archivedAt", "deletedAt"] });
    const space91 = fabricate(ObjectType.SPACE, {
      unset: ["archivedAt", "deletedAt"],
      set: { parentPtr: toNodeRef(package9), id: "space91", orderKey: "a0" },
    });
    let view911 = fabricate(ObjectType.VIEW, {
      unset: ["archivedAt", "deletedAt"],
      set: { parentPtr: toNodeRef(space91), id: "view911", orderKey: "a0" },
    });
    base.extend(package9, space91, view911);
    composite.filter.value = DEFAULT_NODE_FILTER;

    expect(composite.get({ id: package9.id })).toBe(package9);
    expect(composite.get({ id: space91.id })).toBe(space91);
    expect(composite.get({ id: view911.id })).toBe(view911);
    expect(composite.getChildren(package9, NodeType.SPACE)).toEqual([space91]);
    expect(composite.getChildren(space91, NodeType.VIEW)).toEqual([view911]);

    view911 = markDeleted(view911);
    base.update(view911);
    expect(composite.get({ id: view911.id })).toBeNull();
    expect(composite.getChildren(space91, NodeType.VIEW)).toEqual([]);

    view911 = { ...view911, deletedAt: undefined };
    (view911 as any).setPaths = [[ViewProperty.deletedAt.toString()]];
    overlay.update(view911);
    expect(composite.get({ id: view911.id })).toEqual(view911);
    expect(composite.getChildren(space91, NodeType.VIEW)).toEqual([view911]);
  });
});

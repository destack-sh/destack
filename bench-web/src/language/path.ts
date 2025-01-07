import { isSourceNode } from "@/language/const";
import {
  FieldData,
  NodeType,
  ObjectType,
  PathData,
  PathElementData,
  PathElementType,
  PropertyReferenceData,
  SourceNodeData,
  StructType,
} from "@/proto/wire";
import { isNode, isStruct, makeStruct, propertyInfo, toNodeRef } from "@/proto/wiring";
import { supergraph } from "@/system/globals";
import { getNodeName } from "@/ui/icon";

const SIGN_BY_PATH_ELEMENT_TYPE: Partial<Record<PathElementType, string>> = {
  [PathElementType.ROOT]: "/",
  [PathElementType.PARENT]: "..",
  [PathElementType.CURRENT]: ".",
  [PathElementType.CONTEXT]: "$",
};
const PREFIX_BY_PATH_ELEMENT_TYPE: Partial<Record<PathElementType, string>> = {
  [PathElementType.BENCH]: "@",
  [PathElementType.PACKAGE]: "#",
  [PathElementType.CONTAINER]: "~",
  [PathElementType.UNIQUE]: "^",
};

/** Renders a path as a human-readable string. */
export function renderPath(path: PathData): string {
  const pathParts: string[] = [];
  for (const element of path.elements) {
    const sign = SIGN_BY_PATH_ELEMENT_TYPE[element.type];
    if (sign != null) {
      pathParts.push(sign);
    } else if (
      element.type == PathElementType.BENCH ||
      element.type == PathElementType.PACKAGE ||
      element.type == PathElementType.CONTAINER ||
      element.type == PathElementType.UNIQUE ||
      element.type == PathElementType.CHILD
    ) {
      const prefix = PREFIX_BY_PATH_ELEMENT_TYPE[element.type] ?? "";
      pathParts.push(prefix + (element.name ?? "???"));
    } else if (element.type == PathElementType.NODE) {
      const node = element.nodePtr != null ? supergraph.get(element.nodePtr) : null;
      pathParts.push(node != null ? (getNodeName(node) ?? "???") : "???");
    } else if (element.type == PathElementType.RUN) {
      pathParts.push("$>");
    } else if (element.type == PathElementType.ATTRIBUTE) {
      if (element.nodePtr != null) {
        const node = supergraph.get(element.nodePtr);
        pathParts.push(node != null ? (getNodeName(node) ?? "???") : "???");
      } else if (element.propertyPtr != null) {
        const property = propertyInfo(element.propertyPtr.objectType!, element.propertyPtr.id);
        pathParts.push(property.name);
      } else {
        pathParts.push(element.name ?? "??");
      }
    }
  }
  return pathParts.join("/");
}

/** Gets a unique key for the path. */
export function getPathKey(path: PathData): string {
  const pathParts: string[] = [];
  for (const element of path.elements) {
    const typeKey = element.type.toString();
    if (element.nodePtr != null) {
      pathParts.push(`${typeKey}:${element.nodePtr.id!}`);
    } else if (element.propertyPtr != null) {
      pathParts.push(`${typeKey}:${element.propertyPtr.id.toString()}`);
    } else if (element.name != null) {
      pathParts.push(`${typeKey}:${element.name}`);
    } else {
      pathParts.push(`${typeKey}`);
    }
  }
  return pathParts.join(".");
}

export type PathElementIn =
  | (Partial<PathElementData> & Pick<PathElementData, "type">)
  | PathElementType
  | PropertyReferenceData
  | FieldData
  | SourceNodeData;

/** Make a Path from a list of elements. */
export function makePath(...elementsIn: PathElementIn[]): PathData {
  const elements: PathElementData[] = [];
  for (const elementIn of elementsIn) {
    let element;
    if (typeof elementIn == "number") {
      element = { metatype: ObjectType.PATH_ELEMENT, type: elementIn };
    } else if (isStruct(elementIn, StructType.PROPERTY_REFERENCE)) {
      element = { metatype: ObjectType.PATH_ELEMENT, type: PathElementType.ATTRIBUTE, propertyPtr: elementIn };
    } else if (isNode(elementIn, NodeType.FIELD)) {
      element = {
        metatype: ObjectType.PATH_ELEMENT,
        type: PathElementType.ATTRIBUTE,
        nodePtr: toNodeRef(elementIn),
      };
    } else if (isSourceNode(elementIn)) {
      element = { metatype: ObjectType.PATH_ELEMENT, type: PathElementType.NODE, nodePtr: toNodeRef(elementIn) };
    } else {
      element = { metatype: ObjectType.PATH_ELEMENT, ...elementIn };
    }
    elements.push(element);
  }
  const path = makeStruct({ metatype: StructType.PATH, elements });
  return path;
}

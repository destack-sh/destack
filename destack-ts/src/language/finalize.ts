import {
  NODE_DEFINITIONS,
  NodeType,
  STRUCT_DEFINITIONS,
  StructType,
  TRAIT_DEFINITIONS,
  TraitType,
} from "@destack/language";
import {
  NODE_CLASS_BY_TYPE,
  STRUCT_CLASS_BY_TYPE,
  TRAIT_CLASS_BY_TYPE,
} from "@destack/language/registry";

let __isFinalized__ = false;

/** Finalize the Destack language SDK. */
export function finalize(): void {
  if (__isFinalized__) {
    return;
  }
  __isFinalized__ = true;

  // nodes
  for (const nodeDefinition of NODE_DEFINITIONS) {
    const nodeClass = NODE_CLASS_BY_TYPE[nodeDefinition.type];
    if (!nodeClass) {
      throw new Error(
        `Node class not found for type: ${NodeType[nodeDefinition.type]} (have: ${Object.keys(
          NODE_CLASS_BY_TYPE,
        )
          .map((c) => NodeType[c as any])
          .join(", ")})`,
      );
    }
    nodeClass.__definition__ = nodeDefinition;
    nodeClass.__properties__ = {};
    nodeClass.__propertiesById__ = {};
    for (const propertyDefinition of nodeDefinition.properties) {
      nodeClass.__properties__[propertyDefinition.name] = propertyDefinition;
      nodeClass.__propertiesById__[propertyDefinition.id] = propertyDefinition;
    }
  }

  // traits
  for (const traitDefinition of TRAIT_DEFINITIONS) {
    const traitClass = TRAIT_CLASS_BY_TYPE[traitDefinition.type];
    if (!traitClass) {
      throw new Error(
        `Trait class not found for type: ${TraitType[traitDefinition.type]} (have: ${Object.keys(
          TRAIT_CLASS_BY_TYPE,
        )
          .map((c) => TraitType[c as any])
          .join(", ")})`,
      );
    }
    traitClass.__definition__ = traitDefinition;
    traitClass.__properties__ = {};
    traitClass.__propertiesById__ = {};
    for (const propertyDefinition of traitDefinition.properties) {
      traitClass.__properties__[propertyDefinition.name] = propertyDefinition;
      traitClass.__propertiesById__[propertyDefinition.id] = propertyDefinition;
    }
  }

  // structs
  for (const structDefinition of STRUCT_DEFINITIONS) {
    const structClass = STRUCT_CLASS_BY_TYPE[structDefinition.type];
    if (!structClass) {
      throw new Error(
        `Struct class not found for type: ${StructType[structDefinition.type]} (have: ${Object.keys(
          STRUCT_CLASS_BY_TYPE,
        )
          .map((c) => StructType[c as any])
          .join(", ")})`,
      );
    }
    structClass.__definition__ = structDefinition;
    structClass.__properties__ = {};
    structClass.__propertiesById__ = {};
    for (const propertyDefinition of structDefinition.properties) {
      structClass.__properties__[propertyDefinition.name] = propertyDefinition;
      structClass.__propertiesById__[propertyDefinition.id] = propertyDefinition;
    }
  }
}

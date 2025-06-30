import {
  NODE_DEFINITIONS,
  STRUCT_DEFINITIONS,
  TRAIT_DEFINITIONS,
} from "@destack/language/constants";
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
    structClass.__definition__ = structDefinition;
    structClass.__properties__ = {};
    structClass.__propertiesById__ = {};
    for (const propertyDefinition of structDefinition.properties) {
      structClass.__properties__[propertyDefinition.name] = propertyDefinition;
      structClass.__propertiesById__[propertyDefinition.id] = propertyDefinition;
    }
  }
}

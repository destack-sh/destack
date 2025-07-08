import {
  NODE_DEFINITIONS,
  NodeClass,
  NodeDefinition,
  NodeType,
  STRUCT_DEFINITIONS,
  StructClass,
  StructDefinition,
  StructType,
  TRAIT_DEFINITIONS,
  TraitClass,
  TraitDefinition,
  TraitType,
} from "@destack/language";
import {
  NODE_CLASS_BY_TYPE,
  NODE_TYPES_BY_PRIMARY_STORE_TYPE,
  PARENT_TYPES_BY_NODE_TYPE,
  STRUCT_CLASS_BY_TYPE,
  TRAIT_CLASS_BY_TYPE,
} from "@destack/language/registry";
import { Casing, toCasing } from "@destack/utils";

let __isFinalized__ = false;

function _indexProperties(
  cls: NodeClass | TraitClass | StructClass,
  definition: NodeDefinition | TraitDefinition | StructDefinition,
) {
  cls.__properties__ = {};
  cls.__propertiesById__ = {};
  cls.__propertiesByAlias__ = {};
  for (const propertyDefinition of definition.properties) {
    cls.__properties__[propertyDefinition.name] = propertyDefinition;
    cls.__propertiesById__[propertyDefinition.id] = propertyDefinition;
    if (propertyDefinition.isWired) {
      const lowerCamelName = toCasing(propertyDefinition.name, Casing.LOWER_CAMEL);
      const upperCamelName = toCasing(propertyDefinition.name, Casing.CAMEL);
      const aliases = [
        propertyDefinition.name,
        propertyDefinition.name + "_ptr",
        lowerCamelName,
        lowerCamelName + "Ptr",
        upperCamelName,
        upperCamelName + "Ptr",
      ];
      for (const alias of aliases) {
        if (
          cls.__propertiesByAlias__[alias] &&
          cls.__propertiesByAlias__[alias] !== propertyDefinition
        ) {
          throw new Error(
            `property alias conflict: ${propertyDefinition.name} -> ${alias} (have: ${cls.__propertiesByAlias__[alias].name})`,
          );
        }
        cls.__propertiesByAlias__[alias] = propertyDefinition;
      }
    }
  }
}

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

    // properties
    _indexProperties(nodeClass, nodeDefinition);

    // primary store
    for (const storeType of nodeDefinition.primaryStoreTypes) {
      if (!NODE_TYPES_BY_PRIMARY_STORE_TYPE[storeType]) {
        NODE_TYPES_BY_PRIMARY_STORE_TYPE[storeType] = [];
      }
      NODE_TYPES_BY_PRIMARY_STORE_TYPE[storeType].push(nodeDefinition.type);
    }
  }
  // index node parent types
  for (const nodeDefinition of NODE_DEFINITIONS) {
    const parentTypes: NodeType[] = [];
    for (const parentType of nodeDefinition.parentTypes) {
      const parentDefinition = NODE_CLASS_BY_TYPE[parentType].__definition__;
      parentTypes.push(parentType);
      // include subtypes
      for (const parentSubtype of parentDefinition.inheritedBy) {
        if (!parentTypes.includes(parentSubtype)) {
          parentTypes.push(parentSubtype);
        }
      }
    }
    PARENT_TYPES_BY_NODE_TYPE[nodeDefinition.type] = parentTypes;
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
    _indexProperties(traitClass, traitDefinition);
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
    _indexProperties(structClass, structDefinition);
  }
}

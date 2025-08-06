import { JsoncEncoder, JsonEncoder, KompaktEncoder } from "@destack/encoder";
import {
  ENCODERS,
  Encoding,
  loadConstants,
  type NodeClass,
  type NodeDefinition,
  NodeDefinitionReference,
  NodeType,
  ScalarType,
  type StructClass,
  type StructDefinition,
  StructType,
  TraitType,
  Type,
  TypeCardinality,
  Universe,
} from "@destack/language";
import {
  NODE_CLASS_BY_TYPE,
  NODE_TYPE_BY_CLASS,
  NODE_TYPE_SCALAR_BY_TYPE,
  PARENT_TYPES_BY_NODE_TYPE,
  STRUCT_CLASS_BY_TYPE,
  TRAIT_CLASS_BY_TYPE,
} from "@destack/language/registry";
import { Casing, toCasing } from "@destack/utils";

let __isFinalized__ = false;

function _indexProperties(
  cls: NodeClass | StructClass,
  definition: NodeDefinition | StructDefinition,
) {
  cls.__properties__ = {};
  cls.__propertiesById__ = {};
  cls.__propertiesByAlias__ = {};
  for (const propertyDefinition of definition.properties) {
    if (propertyDefinition.name === null) {
      throw new Error(`property name is required for ${definition.name}:${propertyDefinition.id}`);
    }
    cls.__properties__[propertyDefinition.name] = propertyDefinition;
    cls.__propertiesById__[propertyDefinition.id] = propertyDefinition;
    if (propertyDefinition.isWired) {
      const lowerCamelName = toCasing(propertyDefinition.name, Casing.LOWER_CAMEL);
      const upperCamelName = toCasing(propertyDefinition.name, Casing.CAMEL);
      const aliases = [
        propertyDefinition.name,
        `${propertyDefinition.name}_ptr`,
        lowerCamelName,
        `${lowerCamelName}Ptr`,
        upperCamelName,
        `${upperCamelName}Ptr`,
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

/** Finalize the Destack library. */
export function finalize(): void {
  if (__isFinalized__) {
    return;
  }
  __isFinalized__ = true;

  // encoders
  ENCODERS[Encoding.JSON] = new JsonEncoder();
  ENCODERS[Encoding.JSONC] = new JsoncEncoder();
  ENCODERS[Encoding.KOMPAKT] = new KompaktEncoder();
  if (Object.keys(ENCODERS).length !== Object.keys(Encoding).length / 2) {
    throw new Error(
      `missing ${Object.keys(Encoding).length / 2 - Object.keys(ENCODERS).length} encoders`,
    );
  }

  // constants
  loadConstants();

  // nodes
  for (const nodeDefinition of Universe.NODES) {
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
    const nodeDefinitionReference = NodeDefinitionReference.of(nodeClass);
    nodeClass.__definitionReference = nodeDefinitionReference;

    // properties
    _indexProperties(nodeClass, nodeDefinition);
  }

  // index node parent types
  for (const nodeDefinition of Universe.NODES) {
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

  // index node scalar types
  for (const nodeType of NODE_TYPE_BY_CLASS.values()) {
    const scalarType = new Type({
      cardinality: TypeCardinality.SCALAR,
      scalarType: ScalarType.NODE_VALUE,
      nodeTypes: [nodeType],
    });
    NODE_TYPE_SCALAR_BY_TYPE[nodeType] = scalarType;
  }

  // traits
  for (const traitDefinition of Universe.TRAITS) {
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
  }

  // structs
  for (const structDefinition of Universe.STRUCTS) {
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

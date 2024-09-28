/**
 * Many constants are generated into proto/wire, here some additional ones.
 */

import {
  BOUNDARY_STEP_TYPES,
  ENUM_TITLE_BY_TYPE,
  ENUM_TYPES,
  FILTERED_ENUMS,
  getPropertyTitle,
  NODE_SUBTYPE_BY_TYPE,
  PAGE_BLOCK_TYPES,
  RUNNABLE_BLOCK_TYPES,
  toCamelName,
} from "@/language/const";
import {
  getPropertyType,
  makeTypeConstraint,
  makeTypeInfo,
  typeIsNumeric,
  updateFieldType,
  type TypeIdentity,
} from "@/language/field";
import { type ReadNodeGraph } from "@/language/graph";
import { onNodeMorphed } from "@/language/node";
import type { Transaction } from "@/language/transaction";
import {
  BenchType,
  BlockProperty,
  BlockType,
  ENUM_BY_TYPE,
  EnumType,
  EnumTypeMapping,
  FieldProperty,
  FieldZone,
  IconData,
  ModelProvider,
  NodeType,
  ObjectType,
  PipeProperty,
  PrimitiveType,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_TYPE,
  RunOptionsProperty,
  RunProperty,
  StepData,
  StepProperty,
  StepType,
  StructType,
  TypeConstraintProperty,
  TypeInfoProperty,
  TypeKind,
  ViewProperty,
  ViewType,
  type AnyNodeData,
  type BlockData,
  type FieldData,
  type PropertyInfo,
} from "@/proto/wire";
import { isNode, makeStruct } from "@/proto/wiring";
import { ICONS_BY_ENUM_TYPE } from "@/ui/icon";
import { FULL_WIDTH_VIEW_TYPES, getViewForValueType } from "@/ui/view";
import { log } from "@/utils/log";
import { Casing, toCasing } from "@/utils/string";
import type { ViewProps } from "@/views/common";

export type EnumOption<T extends EnumType = EnumType> = {
  id: string;
  icon?: IconData;
  title: string;
  value: EnumTypeMapping[T];
  isHidden?: boolean;
};

const ENUM_OPTIONS_BY_TYPE: Record<EnumType, EnumOption[]> = Object.fromEntries(
  ENUM_TYPES.map((enumType) => [enumType, makeEnumOptions(enumType)]),
) as Record<EnumType, EnumOption[]>;

function makeEnumOptions<T extends EnumType>(enumType: T): EnumOption<T>[] {
  const protoEnum = ENUM_BY_TYPE[enumType];
  const icons = ICONS_BY_ENUM_TYPE[enumType];
  const titles = ENUM_TITLE_BY_TYPE[enumType];
  const availableEnums =
    FILTERED_ENUMS[enumType] ?? Object.values(protoEnum).filter((v) => typeof v == "number" && v > 0);
  const options: EnumOption<T>[] = availableEnums.map((value) => {
    const icon = icons?.[value];
    const name = protoEnum[value] as string;
    if (name == null) throw new Error(`missing enum option ${value} in ${EnumType[enumType]}`);
    const title = titles?.[value] ?? toCasing(name, Casing.CAMEL, true);
    const option: EnumOption<T> = { id: value.toString(), icon, title, value: value as EnumTypeMapping[T] };
    return option;
  });
  return options;
}

export function getEnumOptions<T extends EnumType>(enumType: T): EnumOption<T>[] {
  return ENUM_OPTIONS_BY_TYPE[enumType] as EnumOption<T>[];
}

export function getEnumTitle<T extends EnumType>(enumType: T, enumValue: EnumTypeMapping[T]): string {
  return ENUM_TITLE_BY_TYPE[enumType]?.[enumValue] ?? toCamelName(ENUM_BY_TYPE[enumType], enumValue);
}

/** Gets a random value from an enum, ignoring the number keys (which are for protobuf). */
export function getRandomEnumOption<T extends EnumType>(enumType: T): EnumTypeMapping[T] {
  const options = getEnumOptions(enumType);
  return options[Math.floor(Math.random() * options.length)].value;
}

type InspectionCategory = (
  | { from?: number; to?: number; excluding?: number[] }
  | {
      from: number;
      to: number;
      replace: (properties: PropertyInfo[]) => InspectedPropertyIn;
    }
  | number
  | InspectedProperty
)[];

type InspectedProperty = {
  title: string;
  protoName?: string;
  category: string;
  property: PropertyInfo;
  viewType?: ViewType;
  props?: ViewProps;
  isFullWidth?: boolean;
  read?: (node: AnyNodeData) => any;
  write?: (tx: Transaction, graph: ReadNodeGraph, node: AnyNodeData, value: any) => void;
};
type InspectedPropertyIn = Pick<InspectedProperty, "title" | "viewType" | "props" | "isFullWidth" | "read" | "write">;
type InspectionLayout = {
  properties: InspectedProperty[];
  onWrite?: (tx: Transaction, graph: ReadNodeGraph, node: AnyNodeData, property: PropertyInfo) => void;
};

// NOTE: we (try to) only use metatype/type to avoid recomputing inspection layouts on every change (might have to revisit)
// NOTE :UX :Architecture: inspect view layout & generation is pretty clumsy
const RUNNABLE_TEXT_PROPERTIES = [RunOptionsProperty.modelType];
const RUNNABLE_PROPERTIES = [RunOptionsProperty.cacheMode];
const RUN_OPTIONS_PROPERTIES = PROPERTY_INFOS_BY_TYPE[ObjectType.RUN_OPTIONS]!;
const TYPE_CONSTRAINT_PROPERTIES = PROPERTY_INFOS_BY_TYPE[ObjectType.TYPE_CONSTRAINT]!;
function getInspectionInfo(node: AnyNodeData): Record<string, InspectionCategory> | null {
  if (isNode(node, NodeType.FIELD)) {
    if (node.zone == FieldZone.OPTION) {
      return { Common: [FieldProperty.text] };
    }
    const properties: Record<string, InspectionCategory> = {
      Common: [
        {
          from: 40,
          to: 43,
          replace: () => ({
            title: "Type",
            viewType: ViewType.PICKER,
            props: { valueType: makeTypeInfo({ isRequired: true, benchType: BenchType.TYPE_INFO }) },
            read: (node: AnyNodeData) => node,
            write: (tx: Transaction, graph: ReadNodeGraph, node: AnyNodeData, value: TypeIdentity | null) => {
              updateFieldType(tx, graph, node as FieldData, value);
            },
          }),
        },
        { from: 33, to: 43 },
        { from: 60 },
      ],
      Constraint: [],
    };
    const typeConstraint = PROPERTY_INFOS_BY_TYPE[ObjectType.TYPE_INFO]![TypeInfoProperty.constraint];
    const typeConstraintProperties: TypeConstraintProperty[] = [];
    if (node.primitiveType == PrimitiveType.STRING) {
      typeConstraintProperties.push(TypeConstraintProperty.regex);
      typeConstraintProperties.push(TypeConstraintProperty.startsWith);
      typeConstraintProperties.push(TypeConstraintProperty.endsWith);
    }
    if (node.kind == TypeKind.NODE && node.benchType == null) {
      typeConstraintProperties.push(TypeConstraintProperty.nodeIsAttached);
      typeConstraintProperties.push(TypeConstraintProperty.nodeTypes);
    }
    if (node.benchType == BenchType.BLOCK) {
      typeConstraintProperties.push(TypeConstraintProperty.blockTypes);
    }
    if (node.benchType == BenchType.STEP) {
      typeConstraintProperties.push(TypeConstraintProperty.stepTypes);
    }
    if (node.benchType == BenchType.FILE) {
      typeConstraintProperties.push(TypeConstraintProperty.fileTypes);
      typeConstraintProperties.push(TypeConstraintProperty.fileFormats);
    }
    if (node.benchType == BenchType.VIEW) {
      typeConstraintProperties.push(TypeConstraintProperty.viewTypes);
    }
    if (node.primitiveType == PrimitiveType.STRING || node.isList) {
      typeConstraintProperties.push(TypeConstraintProperty.minLength);
      typeConstraintProperties.push(TypeConstraintProperty.maxLength);
    }
    if (typeIsNumeric(node)) {
      typeConstraintProperties.push(TypeConstraintProperty.minValue);
      typeConstraintProperties.push(TypeConstraintProperty.maxValue);
      typeConstraintProperties.push(TypeConstraintProperty.stepValue);
    }
    typeConstraintProperties
      .map((p) => getNestedInspectedProperty(typeConstraint, "Constraint", TYPE_CONSTRAINT_PROPERTIES[p]))
      .forEach((p) => properties.Constraint.push(p));
    return properties;
  } else if (isNode(node, NodeType.BLOCK)) {
    const properties: Record<string, InspectionCategory> = {
      Common: [BlockProperty.type],
      Run: [],
    };
    if (RUNNABLE_BLOCK_TYPES.includes(node.type) || PAGE_BLOCK_TYPES.includes(node.type)) {
      properties.Run.push(BlockProperty.identityPtr);
      const runOptionProperties = [...RUNNABLE_PROPERTIES];
      if (node.type == BlockType.TEXT) {
        runOptionProperties.push(...RUNNABLE_TEXT_PROPERTIES);
      }
      const runOptions = PROPERTY_INFOS_BY_TYPE[ObjectType.BLOCK]![BlockProperty.runOptions];
      runOptionProperties
        .map((p) => getNestedInspectedProperty(runOptions, "Run", RUN_OPTIONS_PROPERTIES[p]))
        .forEach((p) => properties.Run.push(p));
    }
    if (node.type == BlockType.VALUE) {
      // value type
      properties.Common.push({
        from: BlockProperty.valueType,
        to: BlockProperty.valueType + 1,
        replace: () => ({
          title: "Value Type",
          viewType: ViewType.PICKER,
          props: { valueType: makeTypeInfo({ benchType: BenchType.TYPE_INFO }) },
          read: (node: AnyNodeData) => (node as BlockData).valueType,
          write: (tx: Transaction, graph: ReadNodeGraph, node: AnyNodeData, value: TypeIdentity | null) => {
            const valueType = value == null ? undefined : makeTypeInfo(value);
            tx.update(node as BlockData, { valueType }, { debounce: "tick" });
          },
        }),
      });
    }
    return properties;
  } else if (isNode(node, NodeType.STEP)) {
    const properties: Record<string, InspectionCategory> = {
      Common: [StepProperty.type],
      Run: [],
    };
    if ([StepType.BLOCK, StepType.TRIGGER].includes(node.type)) {
      properties.Common.push(StepProperty.nodePtr);
    }
    if (!BOUNDARY_STEP_TYPES.includes(node.type)) {
      properties.Run.push(StepProperty.identityPtr);
      const stepProperties = PROPERTY_INFOS_BY_TYPE[ObjectType.STEP]!;
      const runOptionProperties = [...RUNNABLE_PROPERTIES];
      if (node.type == StepType.TEXT) {
        runOptionProperties.push(...RUNNABLE_TEXT_PROPERTIES);
      }
      runOptionProperties.push(RunOptionsProperty.suppressFail);
      runOptionProperties.push(RunOptionsProperty.suppressAbort);
      runOptionProperties
        .map((p) =>
          getNestedInspectedProperty(stepProperties[StepProperty.runOptions], "Run", RUN_OPTIONS_PROPERTIES[p]),
        )
        .forEach((p) => properties.Run.push(p));
    }
    return properties;
  } else if (isNode(node, NodeType.PIPE)) {
    const properties: Record<string, InspectionCategory> = {
      Common: [PipeProperty.type, PipeProperty.color, PipeProperty.isHidden],
      Filter: [PipeProperty.filter],
      Mapping: [PipeProperty.modulation, PipeProperty.combinator],
    };
    return properties;
  } else if (isNode(node, NodeType.VIEW)) {
    const properties = {
      Common: [{ to: 40 }, ViewProperty.isInput],
      Content: [{ from: 40, to: 50 }],
      Style: [{ from: 50, to: 60 }],
      Layout: [{ from: 60, to: 70 }],
      Behavior: [{ from: 70, to: 80 }],
    };
    return properties;
  } else {
    return null;
  }
}

const EXCLUDED_PROPERTIES: string[] = ["order_key"];

/** Generates inspection property info for a nested property. */
function getNestedInspectedProperty(
  parentProperty: PropertyInfo,
  category: string,
  property: PropertyInfo,
): InspectedProperty {
  const inspectedProperty = getInspectedProperty(parentProperty.component, category, property);
  const parentPropertyName = PROPERTY_ENUM_BY_TYPE[parentProperty.component]![parentProperty.id];
  const propertyName = PROPERTY_ENUM_BY_TYPE[property.component]![property.id];
  return {
    ...inspectedProperty,
    read: (node: AnyNodeData) => (node as any)[parentPropertyName]?.[propertyName],
    write: (tx: Transaction, graph: ReadNodeGraph, node: AnyNodeData, value: any) => {
      if (parentProperty.referenceStruct != null) {
        value = makeStruct({
          metatype: parentProperty.referenceStruct,
          ...(node as any)[parentPropertyName],
          [propertyName]: value,
        });
      }
      tx.update(node as any, { [parentPropertyName]: value }, { debounce: "tick" });
    },
  };
}

/** Generate inspection property info for a property. */
function getInspectedProperty(metatype: ObjectType, category: string, property: PropertyInfo): InspectedProperty {
  const allProperties = PROPERTY_ENUM_BY_TYPE[metatype] ?? [];
  const title = getPropertyTitle(property);
  const protoName = allProperties[property.id];
  const inspectedProperty: InspectedProperty = { title, protoName, category, property };
  const propertyType = getPropertyType(property);
  const valueView = getViewForValueType(propertyType);
  if (valueView == null) {
    throw new Error(`no view for ${ObjectType[metatype]}.${property.id}`);
  }
  inspectedProperty.viewType = valueView.type;
  inspectedProperty.props = valueView;
  inspectedProperty.isFullWidth = FULL_WIDTH_VIEW_TYPES.includes(valueView.type!);
  return inspectedProperty;
}

/** Generates the inspection layout for an object metatype. */
export function getInspectionLayout(node: AnyNodeData, options?: { exclude?: string[] }): InspectionLayout {
  const propertyInfos = PROPERTY_INFOS_BY_TYPE[node.metatype];
  const seenProperties: Record<number, PropertyInfo> = {};
  const inspectedProperties: InspectedProperty[] = [];
  const excluded = EXCLUDED_PROPERTIES.concat(options?.exclude ?? []);

  const categories: Record<string, InspectionCategory> = getInspectionInfo(node) ?? {
    Common: [{ from: undefined, to: undefined }],
  };
  for (const category of Object.keys(categories)) {
    const categoryProperties = categories[category as keyof typeof categories];
    // assemble all properties in category
    for (const range of categoryProperties) {
      let propertiesInRange: PropertyInfo[];
      if (typeof range == "object" && "title" in range) {
        propertiesInRange = [];
      } else if (typeof range == "object" && !("title" in range)) {
        propertiesInRange = Object.values(propertyInfos).filter((property) => {
          if ((range.from != null && property.id < range.from) || (range.to != null && property.id >= range.to)) {
            return false;
          } else if ("excluding" in range && range.excluding != null && range.excluding.includes(property.id)) {
            return false;
          } else {
            return true;
          }
        });
      } else {
        propertiesInRange = Object.values(propertyInfos).filter((property) => property.id == range);
      }

      // filter & map
      if (typeof range == "object" && "replace" in range) {
        for (const property of propertiesInRange) {
          seenProperties[property.id] = property;
        }
        const replaced = range.replace(propertiesInRange);
        const inspectedProperty: InspectedProperty = { ...replaced, property: propertiesInRange[0], category };
        inspectedProperties.push(inspectedProperty);
        continue; // already handled
      } else if (typeof range == "object" && "title" in range) {
        inspectedProperties.push(range);
        continue; // already handled
      }
      for (const property of propertiesInRange) {
        if (seenProperties[property.id]) continue;
        if (property.id < 30 || property.isAutoset || property.isComputed || property.isSystem) continue;
        if (excluded.includes(property.name)) continue;
        seenProperties[property.id] = property;

        // map properties to components
        try {
          const inspectedProperty = getInspectedProperty(node.metatype, category, property);
          inspectedProperties.push(inspectedProperty);
        } catch (e) {
          log.warn("lang.missingView", property); // will indicate no view for value in UI
          continue;
        }
      }
    }
  }

  const discriminator = NODE_SUBTYPE_BY_TYPE[node.metatype as unknown as NodeType];
  function onWrite(tx: Transaction, graph: ReadNodeGraph, node: AnyNodeData, property: PropertyInfo) {
    // trigger morph
    if (discriminator == property.name) {
      onNodeMorphed(tx, graph, node);
    }
  }

  return { properties: inspectedProperties, onWrite };
}

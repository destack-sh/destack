/**
 * Many constants are generated into proto/wire, here some additional ones.
 */

import { type ReadNodeGraph } from "@/language/graph";
import type { Transaction } from "@/language/transaction";
import {
  Anchor,
  BenchType,
  BlockProperty,
  BlockType,
  EditType,
  ENUM_BY_TYPE,
  EnumType,
  FieldProperty,
  FieldZone,
  IconData,
  NodeType,
  ObjectType,
  PrimitiveType,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_TYPE,
  StructType,
  ViewProperty,
  ViewType,
  type AnyNodeData,
  type BlockData,
  type EnumTypeMapping,
  type FieldData,
  type PropertyInfo,
} from "@/proto/wire";
import { onNodeMorphed } from "@/language/node";
import { ICONS_BY_ENUM_TYPE } from "@/ui/icon";
import { getViewForValueType } from "@/ui/view";
import { log } from "@/utils/log";
import { toCasing, Casing } from "@/utils/string";
import type { ViewProps } from "@/views/common";
import { ENUM_TYPES, NODE_SUBTYPE_BY_TYPE, NODE_TYPES, PAGE_BLOCK_TYPES, RUNNABLE_BLOCK_TYPES, toCamelName } from "@/language/const";
import { makeTypeInfo, type TypeIdentity } from "@/language/field";

//
// Enums
//

export const EDIT_TYPE_PRESENT_VERB: Record<EditType, string> = {
  [EditType.UNSPECIFIED]: "???",
  [EditType.CREATE]: "creates",
  [EditType.UPSERT]: "upserts",
  [EditType.UPDATE]: "updates",
  [EditType.MOVE]: "moves",
  [EditType.ARCHIVE]: "archives",
  [EditType.UNARCHIVE]: "unarchives",
  [EditType.DELETE]: "deletes",
  [EditType.RESTORE]: "restores",
  [EditType.ERASE]: "erases",
};
export const EDIT_TYPE_PAST_VERB: Record<EditType, string> = {
  [EditType.UNSPECIFIED]: "???",
  [EditType.CREATE]: "created",
  [EditType.UPSERT]: "upserted",
  [EditType.UPDATE]: "updated",
  [EditType.MOVE]: "moved",
  [EditType.ARCHIVE]: "archived",
  [EditType.UNARCHIVE]: "unarchived",
  [EditType.DELETE]: "deleted",
  [EditType.RESTORE]: "restored",
  [EditType.ERASE]: "erased",
};

// NOTE: we soft-limit the subset of available enum options in bench-web
//  (in code and backend the entire ranges are available)
export const EXPOSED_BLOCK_TYPES = [
  BlockType.PAGE,
  BlockType.CLASS,
  BlockType.CHOICE,
  BlockType.TEXT,
  BlockType.CODE,
  BlockType.VARIABLE,
  BlockType.IDENTITY,
];
export const EXPOSED_STRUCT_TYPES = [
  // core
  StructType.PATH,
  StructType.TYPE_INFO,
  StructType.SCHEDULE,
  StructType.TRIGGER_INFO,
  // files
  StructType.ICON,
  // code
  StructType.CODE,
  // expressions
  StructType.EXPRESSION,
  StructType.SELECTION,
  // views
  StructType.COLOR,
  StructType.FONT,
  StructType.OFFSET,
  StructType.BOX,
  // access
  StructType.POLICY,
  StructType.POLICY_RULE,
  // flow
  StructType.PIPE,
  // text
  StructType.TEXT,
  // run
  StructType.RUN_OPTIONS,
  StructType.RUN_ATTEMPT,
  StructType.RUN_ERROR,
  StructType.RUN_FRAME,
  StructType.RUN_TRACE,
  StructType.BREAKPOINT,
  StructType.MODEL_OPTIONS,
  StructType.LOG_INFO,
];
export const EXPOSED_PRIMITIVE_TYPES = [
  PrimitiveType.BOOLEAN,
  PrimitiveType.INT64,
  PrimitiveType.FLOAT64,
  PrimitiveType.STRING,
  PrimitiveType.JSON,
  PrimitiveType.BYTES,
  PrimitiveType.UUID,
  PrimitiveType.DATETIME,
];
export const EXPOSED_ANCHORS = [
  // the rest are exposed too but as additional flags (start/end)
  Anchor.LEFT,
  Anchor.TOP,
  Anchor.RIGHT,
  Anchor.BOTTOM,
];
export const FILTERED_ENUMS: Partial<Record<EnumType, number[]>> = {
  [EnumType.BLOCK_TYPE]: EXPOSED_BLOCK_TYPES,
  [EnumType.STRUCT_TYPE]: EXPOSED_STRUCT_TYPES,
  [EnumType.OBJECT_TYPE]: [...NODE_TYPES, ...EXPOSED_STRUCT_TYPES],
  [EnumType.BENCH_TYPE]: [...NODE_TYPES, ...EXPOSED_STRUCT_TYPES, ...ENUM_TYPES],
  [EnumType.PRIMITIVE_TYPE]: EXPOSED_PRIMITIVE_TYPES,
  [EnumType.ANCHOR]: EXPOSED_ANCHORS,
};
export const ENUM_TITLE_BY_TYPE: Partial<Record<EnumType, Record<any, string>>> = {
  [EnumType.PRIMITIVE_TYPE]: {
    [PrimitiveType.INT64]: "Integer",
    [PrimitiveType.FLOAT64]: "Number",
  },
};

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

export function getPropertyTitle(property: PropertyInfo): string {
  let pythonName = property.name;
  if (pythonName.endsWith("_ptr")) pythonName = pythonName.slice(0, -4);
  if (pythonName.endsWith("_packed")) pythonName = pythonName.slice(0, -7);
  const title = toCasing(pythonName, Casing.CAMEL, true);
  return title;
}

//
// Inspection
//

type InspectionCategory = (
  | { from?: number; to?: number; excluding?: number[] }
  | {
      from: number;
      to: number;
      replace: (properties: PropertyInfo[]) => InspectedPropertyIn;
    }
  | number
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
  write?: (tx: Transaction, node: AnyNodeData, value: any) => void;
};
type InspectedPropertyIn = Pick<InspectedProperty, "title" | "viewType" | "props" | "isFullWidth" | "read" | "write">;
type InspectionLayout = {
  properties: InspectedProperty[];
  onWrite?: (tx: Transaction, graph: ReadNodeGraph, node: AnyNodeData, property: PropertyInfo) => void;
};

// NOTE: we (try to) only use metatype/type to avoid recomputing inspection layouts on every change (might have to revisit)
// NOTE :Architecture: the inspection layout generation is a bit clumsy
function getInspectionInfo(metatype: ObjectType, type: any): Record<string, InspectionCategory> | null {
  if (metatype == ObjectType.FIELD) {
    if (type == FieldZone.OPTION) {
      return { Common: [FieldProperty.zone, FieldProperty.text] };
    }
    const properties = {
      Common: [
        FieldProperty.zone,
        {
          from: 40,
          to: 43,
          replace: () => ({
            title: "Type",
            viewType: ViewType.PICKER,
            props: { valueType: makeTypeInfo({ isRequired: true, benchType: BenchType.TYPE_INFO }) },
            read: (node: FieldData) => node,
            write: (tx: Transaction, node: FieldData, value: TypeIdentity | null) => {
              tx.update(
                node,
                {
                  kind: value?.kind,
                  primitiveType: value?.primitiveType,
                  benchType: value?.benchType,
                  baseTypePtr: value?.baseTypePtr,
                  constraint: value?.constraint,
                },
                { debounce: "tick" },
              );
            },
          }),
        },
        { from: 30, to: 43, excluding: [FieldProperty.valuePacked] },
      ],
      Constraint: [{ from: 60 }],
    };
    return properties;
  } else if (metatype == ObjectType.BLOCK) {
    const properties: Record<string, InspectionCategory> = {
      Common: [BlockProperty.type],
      Run: [],
    };
    if (RUNNABLE_BLOCK_TYPES.includes(type) || PAGE_BLOCK_TYPES.includes(type)) {
      properties.Run.push(BlockProperty.identityPtr);
      properties.Run.push(BlockProperty.isPaused);
    }
    if (type == BlockType.VARIABLE) {
      // value type
      properties.Common.push({
        from: BlockProperty.valueType,
        to: BlockProperty.valueType + 1,
        replace: () => ({
          title: "Value Type",
          viewType: ViewType.PICKER,
          props: { valueType: makeTypeInfo({ benchType: BenchType.TYPE_INFO }) },
          read: (node: AnyNodeData) => (node as BlockData).valueType,
          write: (tx: Transaction, node: AnyNodeData, value: TypeIdentity | null) => {
            const valueType =
              value == null
                ? undefined
                : makeTypeInfo({
                    kind: value.kind,
                    primitiveType: value.primitiveType,
                    benchType: value.benchType,
                    baseTypePtr: value.baseTypePtr,
                    constraint: value.constraint,
                  });
            tx.update(node as BlockData, { valueType }, { debounce: "tick" });
          },
        }),
      });
    }
    return properties;
  } else if (metatype == ObjectType.VIEW) {
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

export const FULL_WIDTH_VIEW_TYPES = [ViewType.TEXT, ViewType.CODE, ViewType.IMAGE, ViewType.AUDIO, ViewType.VIDEO];
const ALWAYS_EXCLUDED_PROPERTIES: string[] = ["order_key"];

/** Generates the inspection layout for an object metatype. */
export function getInspectionLayout(
  metatype: ObjectType,
  type: any,
  options?: { exclude?: string[] },
): InspectionLayout {
  const propertyInfos = PROPERTY_INFOS_BY_TYPE[metatype];
  const seenProperties: Record<number, PropertyInfo> = {};
  const inspectedProperties: InspectedProperty[] = [];
  const excluded = ALWAYS_EXCLUDED_PROPERTIES.concat(options?.exclude ?? []);

  const allProperties = PROPERTY_ENUM_BY_TYPE[metatype] ?? [];
  const categories: Record<string, InspectionCategory> = getInspectionInfo(metatype, type) ?? {
    Common: [{ from: undefined, to: undefined }],
  };
  for (const category of Object.keys(categories)) {
    const categoryProperties = categories[category as keyof typeof categories];
    // assemble all properties in category
    for (const range of categoryProperties) {
      let propertiesInRange;
      if (typeof range == "object") {
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
      }
      for (const property of propertiesInRange) {
        if (seenProperties[property.id]) continue;
        if (property.id < 30 || property.isAutoset || property.isComputed || property.isSystem) continue;
        if (excluded.includes(property.name)) continue;
        seenProperties[property.id] = property;

        // map properties to components
        const title = getPropertyTitle(property);
        const protoName = allProperties[property.id];
        const inspectedProperty: InspectedProperty = { title, protoName, category, property };
        const valueView = getViewForValueType({
          primitiveType: property.primitiveType,
          benchType: (property.enumType ?? property.referenceNodes?.[0] ?? property.referenceStruct) as unknown as
            | BenchType
            | undefined,
          isRequired: property.isRequired ?? false,
          isList: property.isList ?? false,
          isSecret: property.isEncrypted ?? false,
          constraint:
            property.constraint != null ? { metatype: ObjectType.TYPE_CONSTRAINT, ...property.constraint } : undefined,
        });
        if (valueView == null) {
          log.warn("lang.missingView", property); // will indicate no view for value in UI
          continue;
        }
        inspectedProperty.viewType = valueView.viewType;
        inspectedProperty.props = valueView.props;
        inspectedProperty.isFullWidth = FULL_WIDTH_VIEW_TYPES.includes(valueView.viewType);
        inspectedProperties.push(inspectedProperty);
      }
    }
  }

  const discriminator = NODE_SUBTYPE_BY_TYPE[metatype as unknown as NodeType];
  function onWrite(tx: Transaction, graph: ReadNodeGraph, node: AnyNodeData, property: PropertyInfo) {
    // trigger morph
    if (discriminator == property.name) {
      onNodeMorphed(tx, graph, node);
    }
  }

  return { properties: inspectedProperties, onWrite };
}

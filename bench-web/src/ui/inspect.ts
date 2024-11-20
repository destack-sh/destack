import { getPropertyTitle, RUNNABLE_BLOCK_TYPES } from "@/language/const";
import { getPropertyType, TypeIdentity } from "@/language/field";
import { ReadNodeGraph } from "@/language/graph";
import { unpackSubnode } from "@/language/node";
import { Transaction } from "@/language/transaction";
import {
  ActionBlockData,
  ActionBlockProperty,
  AnyNodeData,
  BlockProperty,
  BlockType,
  FieldProperty,
  FieldType,
  NodeType,
  PipeProperty,
  PROPERTY_ENUM_BY_SUBTYPE,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_SUBTYPE,
  PROPERTY_INFOS_BY_TYPE,
  PropertyInfo,
  StepProperty,
  ViewType,
} from "@/proto/wire";
import { isNode } from "@/proto/wiring";
import { FULL_WIDTH_VIEW_TYPES, getView } from "@/ui/view";

export type InspectLayout = {
  sections: InspectSection[];
};

export type InspectSection = {
  title?: string;
  rows: InspectRow[];
};

type InspectRowBase = {
  title?: string;
};
export type InspectFieldsRow = InspectRowBase & {
  type: "fields";
  fieldType: FieldType;
};
export type InspectPropertyRow = InspectRowBase & {
  type: "property";
  title: string;
  prop: PropertyInfo;
  propType: TypeIdentity;
  isInput: boolean;
  isFullWidth: boolean;
  viewType: ViewType;
  viewProps: any;
  read: () => any;
  write: (tx: Transaction, graph: ReadNodeGraph, value: any) => void;
};
export type InspectRow = InspectFieldsRow | InspectPropertyRow;

export function makeInspectLayout(node: AnyNodeData): InspectLayout {
  const sections: InspectSection[] = [];

  const metatype = node.metatype as unknown as NodeType;
  const propertyEnum = PROPERTY_ENUM_BY_TYPE[node.metatype]!;
  const propertyInfos = PROPERTY_INFOS_BY_TYPE[node.metatype]!;
  const subtype = (node as any).type;
  const subpropertyEnum = subtype != null ? PROPERTY_ENUM_BY_SUBTYPE[metatype]?.[subtype] : undefined;
  const subpropertyInfos = PROPERTY_INFOS_BY_SUBTYPE[metatype]?.[subtype];
  const subnode =
    subtype != null && node.subnodePacked != null ? unpackSubnode(metatype, subtype, node.subnodePacked) : null;

  function addSection(title: string | undefined, rows: InspectRow[]) {
    sections.push({ title, rows });
  }

  function propertyRow(property: number, options?: { isDisabled?: boolean }): InspectPropertyRow {
    const prop = propertyInfos[property];
    const propKey = propertyEnum[property];
    const title = getPropertyTitle(prop);
    const propType = getPropertyType(prop);
    const view = getView(propType);
    if (view == null) throw new Error(`no view for property type: ${title}`);
    return {
      type: "property",
      title,
      prop,
      propType,
      isInput: !options?.isDisabled,
      isFullWidth: FULL_WIDTH_VIEW_TYPES.includes(view.type!),
      viewType: view.type!,
      viewProps: view,
      read: () => (node as any)[propKey],
      write: (tx, graph, value) => {
        // nocheckin
        // not sure how to :DebounceNestedValue properly (different types with different debounce needs)
      },
    };
  }

  function subpropertyRow(
    property: number,
    options?: { isDisabled?: boolean; isFullWidth?: boolean },
  ): InspectPropertyRow {
    if (subpropertyEnum == null || subpropertyInfos == null) throw new Error("no subproperties");
    const prop = subpropertyInfos[property];
    const propKey = subpropertyEnum[property];
    const title = getPropertyTitle(prop);
    const propType = getPropertyType(prop);
    const view = getView(propType);
    if (view == null) throw new Error(`no view for property type: ${title}`);
    return {
      type: "property",
      title,
      prop,
      propType,
      isInput: !options?.isDisabled,
      isFullWidth: options?.isFullWidth || FULL_WIDTH_VIEW_TYPES.includes(view.type!),
      viewType: view.type!,
      viewProps: view,
      read: () => (subnode as any)?.[propKey],
      write: (tx, graph, value) => {
        // nocheckin
        // not sure how to :DebounceNestedValue properly (different types with different debounce needs)
      },
    };
  }

  //
  // Blocks
  //

  if (isNode(node, NodeType.BLOCK)) {
    if (node.type != BlockType.TEXT) {
      addSection(undefined, [propertyRow(BlockProperty.type, { isDisabled: true }), propertyRow(BlockProperty.text)]);
    }

    if (node.type == BlockType.CHOICE) {
      addSection("Options", [{ type: "fields", fieldType: FieldType.OPTION }]);
    } else if (node.type == BlockType.DATABASE || node.type == BlockType.CLASS || node.type == BlockType.MESSAGE) {
      addSection("Members", [{ type: "fields", fieldType: FieldType.MEMBER }]);
    } else if (RUNNABLE_BLOCK_TYPES.includes(node.type)) {
      addSection("Variables", [{ type: "fields", fieldType: FieldType.VARIABLE }]);
      addSection("Schema", [
        { type: "fields", title: "Inputs", fieldType: FieldType.INPUT },
        { type: "fields", title: "Outputs", fieldType: FieldType.OUTPUT },
      ]);
    }

    if (node.type == BlockType.ACTION) {
      addSection("Run", [subpropertyRow(ActionBlockProperty.mode), subpropertyRow(ActionBlockProperty.toolsPtr)]);
    }
  }

  //
  // Fields
  //
  else if (isNode(node, NodeType.FIELD)) {
    addSection(undefined, [propertyRow(FieldProperty.type, { isDisabled: true }), propertyRow(FieldProperty.text)]);
  }

  //
  // Steps
  //
  else if (isNode(node, NodeType.STEP)) {
    addSection(undefined, [propertyRow(StepProperty.type, { isDisabled: true }), propertyRow(StepProperty.text)]);
  }

  //
  // Pipes
  //
  else if (isNode(node, NodeType.PIPE)) {
    addSection(undefined, [propertyRow(PipeProperty.type, { isDisabled: true }), propertyRow(PipeProperty.text)]);
  }

  return {
    sections,
  };
}

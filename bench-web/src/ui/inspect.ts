import { BOUNDARY_STEP_TYPES, getPropertyTitle, RUNNABLE_BLOCK_TYPES, toCamelName } from "@/language/const";
import { createField, getPropertyType, makeTypeInfo, TypeIdentity } from "@/language/field";
import { ReadNodeGraph } from "@/language/graph";
import { unpackSubnode } from "@/language/node";
import { Transaction } from "@/language/transaction";
import {
  ActionBlockData,
  ActionBlockProperty,
  ActionMode,
  AnyNodeData,
  BenchType,
  BlockData,
  BlockProperty,
  BlockType,
  FieldProperty,
  FieldType,
  IconData,
  NodeType,
  PipeProperty,
  PROPERTY_ENUM_BY_SUBTYPE,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_SUBTYPE,
  PROPERTY_INFOS_BY_TYPE,
  PropertyInfo,
  StepProperty,
  StepType,
  ViewType,
} from "@/proto/wire";
import { isNode } from "@/proto/wiring";
import { ICON_BY_FIELD_TYPE, makeIcon } from "@/ui/icon";
import { pushPopover } from "@/ui/popover";
import { FULL_WIDTH_VIEW_TYPES, getView } from "@/ui/view";

export type InspectLayout = {
  sections: InspectSection[];
};

export type InspectAction = {
  title: string;
  icon: IconData;
  action: (e: MouseEvent) => void;
};
export type InspectSection = {
  title?: string;
  rows: InspectRow[];
  isDefaultCollapsed?: boolean;
  summary?: string;
  actions?: InspectAction[];
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
  write: (value: any) => void;
};
export type InspectIconRow = InspectRowBase & {
  type: "icon";
  icon: IconData;
};
export type InspectRow = InspectFieldsRow | InspectPropertyRow | InspectIconRow;

export function makeInspectLayout(
  node: AnyNodeData,
  graph: ReadNodeGraph,
  txFactory: () => Transaction,
): InspectLayout {
  const sections: InspectSection[] = [];

  const metatype = node.metatype as unknown as NodeType;
  const propertyEnum = PROPERTY_ENUM_BY_TYPE[node.metatype]!;
  const propertyInfos = PROPERTY_INFOS_BY_TYPE[node.metatype]!;
  const subtype = (node as any).type;
  const subpropertyEnum = subtype != null ? PROPERTY_ENUM_BY_SUBTYPE[metatype]?.[subtype] : undefined;
  const subpropertyInfos = PROPERTY_INFOS_BY_SUBTYPE[metatype]?.[subtype];
  const subnode =
    subtype != null && node.subnodePacked != null
      ? (unpackSubnode(metatype, subtype as never, node.subnodePacked) as any)
      : null;

  /** Make a Section */
  function section(
    title: string | undefined,
    rows: InspectRow[],
    options?: { isDefaultCollapsed?: boolean; summary?: string; actions?: InspectAction[] },
  ) {
    sections.push({
      title,
      rows,
      isDefaultCollapsed: options?.isDefaultCollapsed,
      summary: options?.summary,
      actions: options?.actions,
    });
  }

  /** Make a property, subproperty or nested property row */
  function rowProperty(
    property: number,
    options?: { isFullWidth?: boolean; isDisabled?: boolean },
  ): InspectPropertyRow {
    let prop: PropertyInfo;
    let propKey: string;
    if (propertyInfos[property] != null) {
      prop = propertyInfos[property];
      propKey = propertyEnum[property];
    } else if (subpropertyInfos?.[property] != null) {
      prop = subpropertyInfos[property];
      propKey = subpropertyEnum![property];
    } else {
      throw new Error(`no property info for: ${property}`);
    }
    if (prop == null) throw new Error(`no property info for: ${property} ${propKey}`);
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
      read: () => (node as any)[propKey],
      write: (value) => {
        // nocheckin
        // not sure how to :DebounceNestedValue properly (different types with different debounce needs)
      },
    };
  }

  function actionAddField(fieldType: FieldType, icon: IconData | string = "fas fa-plus"): InspectAction {
    return {
      title: `Add ${toCamelName(FieldType, fieldType)}`,
      icon: makeIcon(icon),
      action: (e) => onAddFieldAction(e, fieldType, node as BlockData, graph, txFactory),
    };
  }

  function sectionSchema() {
    section(
      "Schema",
      [
        { type: "fields", fieldType: FieldType.INPUT },
        { type: "icon", icon: makeIcon("fas fa-arrow-down") },
        { type: "fields", fieldType: FieldType.OUTPUT },
      ],
      {
        actions: [
          actionAddField(FieldType.INPUT, ICON_BY_FIELD_TYPE[FieldType.INPUT]),
          actionAddField(FieldType.OUTPUT, ICON_BY_FIELD_TYPE[FieldType.OUTPUT]),
        ],
      },
    );
  }

  function sectionAction() {
    const mode: ActionMode = (subnode as ActionBlockData)?.mode ?? ActionMode.ADAPTIVE;
    const rows: InspectRow[] = [rowProperty(ActionBlockProperty.mode)];
    if (mode == ActionMode.STRICT) {
      rows.push(rowProperty(ActionBlockProperty.delegatePtr));
      rows.push(rowProperty(ActionBlockProperty.code, { isFullWidth: true }));
    } else {
      rows.push(rowProperty(ActionBlockProperty.toolsPtr));
    }
    section("Action", rows, {
      summary: toCamelName(ActionMode, mode),
    });
  }

  function sectionRun() {
    section("Run", [], { isDefaultCollapsed: true });
  }

  //
  // Blocks
  //

  if (isNode(node, NodeType.BLOCK)) {
    if (node.type != BlockType.TEXT) {
      section(undefined, [rowProperty(BlockProperty.text)]);
    }

    if (node.type == BlockType.CHOICE) {
      section("Options", [{ type: "fields", fieldType: FieldType.OPTION }], {
        actions: [actionAddField(FieldType.OPTION)],
      });
    } else if (node.type == BlockType.DATABASE || node.type == BlockType.CLASS || node.type == BlockType.MESSAGE) {
      section("Members", [{ type: "fields", fieldType: FieldType.MEMBER }], {
        actions: [actionAddField(FieldType.MEMBER)],
      });
    } else if (RUNNABLE_BLOCK_TYPES.includes(node.type)) {
      section("Variables", [{ type: "fields", fieldType: FieldType.VARIABLE }], {
        actions: [actionAddField(FieldType.VARIABLE)],
      });
      sectionSchema();
    }
    if (node.type == BlockType.ACTION) {
      sectionAction();
    }
    if (RUNNABLE_BLOCK_TYPES.includes(node.type)) {
      sectionRun();
    }
  }

  //
  // Fields
  //
  else if (isNode(node, NodeType.FIELD)) {
    section(undefined, [rowProperty(FieldProperty.text)]);
  }

  //
  // Steps
  //
  else if (isNode(node, NodeType.STEP)) {
    section(undefined, [rowProperty(StepProperty.text)]);
    if (!BOUNDARY_STEP_TYPES.includes(node.type)) {
      sectionSchema();
    }
    if (node.type == StepType.ACTION) {
      sectionAction();
    }
    sectionRun();
  }

  //
  // Pipes
  //
  else if (isNode(node, NodeType.PIPE)) {
    section(undefined, [
      rowProperty(PipeProperty.text),
      rowProperty(PipeProperty.type),
      rowProperty(PipeProperty.color),
    ]);
    sectionRun();
  }

  return {
    sections,
  };
}

/** Handle an 'add Field' button (either directly or by spawning a Popover) */
export function onAddFieldAction(
  e: MouseEvent,
  fieldType: FieldType,
  parent: BlockData,
  graph: ReadNodeGraph,
  txFactory: () => Transaction,
) {
  if (fieldType == FieldType.OPTION) {
    createField(txFactory(), graph, {
      anchor: "inside",
      target: parent!,
      field: { type: fieldType },
    });
  } else {
    const button = (e.target as HTMLElement).closest("button")!;
    pushPopover({
      trigger: button,
      reference: button,
      info: {
        component: ViewType.PICKER,
        placement: "bottom-left",
        offset: "referenceWidth",
        props: { valueType: makeTypeInfo({ benchType: BenchType.TYPE_INFO }) },
        onApply: (typeInfo: TypeIdentity) => {
          createField(txFactory(), graph, {
            anchor: "inside",
            target: parent,
            field: { ...typeInfo, type: fieldType },
          });
        },
      },
    });
  }
}

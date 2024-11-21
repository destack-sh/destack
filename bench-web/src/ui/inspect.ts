import {
  BOUNDARY_STEP_TYPES,
  getPropertyTitle,
  isStructType,
  RUNNABLE_BLOCK_TYPES,
  toCamelName,
} from "@/language/const";
import { createField, getPropertyType, makeTypeInfo, TypeIdentity, updateFieldType } from "@/language/field";
import { ReadNodeGraph } from "@/language/graph";
import { unpackSubnode } from "@/language/node";
import { makeEdit, makeEditFromSubnode, Transaction, TransactionOptions } from "@/language/transaction";
import {
  ActionBlockData,
  ActionBlockProperty,
  ActionMode,
  AnyNodeData,
  BenchType,
  BlockData,
  BlockProperty,
  BlockType,
  FieldData,
  FieldProperty,
  FieldType,
  IconData,
  NodeType,
  ObjectType,
  PipeProperty,
  PROPERTY_ENUM_BY_SUBTYPE,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_SUBTYPE,
  PROPERTY_INFOS_BY_TYPE,
  PropertyInfo,
  RunOptionsProperty,
  StepProperty,
  StepType,
  TypeKind,
  Variant,
  ViewType,
} from "@/proto/wire";
import { isNode, makeStruct } from "@/proto/wiring";
import { ICON_BY_FIELD_TYPE, makeIcon } from "@/ui/icon";
import { pushPopover } from "@/ui/popover";
import { FULL_WIDTH_VIEW_TYPES, getView } from "@/ui/view";
import { assertNever } from "@/utils/functools";
import { ViewProps } from "@/views/common";

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
export type InspectViewRow = InspectRowBase & {
  type: "view";
  isFullWidth: boolean;
  viewType: ViewType;
  viewProps: ViewProps;
  read: () => any;
  write: (value: any) => void;
};
export type InspectIconRow = InspectRowBase & {
  type: "icon";
  icon: IconData;
};
export type InspectRow = InspectFieldsRow | InspectViewRow | InspectIconRow;

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

  /** Gets a top-level property with the given key */
  function property(property: number): { prop: PropertyInfo; propKey: string; isSubnode: boolean } {
    let prop: PropertyInfo;
    let propKey: string;
    if (propertyInfos[property] != null) {
      prop = propertyInfos[property];
      propKey = propertyEnum[property];
      if (prop == null) throw new Error(`no property info for: ${property}`);
      return { prop, propKey, isSubnode: false };
    } else if (subpropertyInfos?.[property] != null) {
      prop = subpropertyInfos[property];
      propKey = subpropertyEnum![property];
      if (prop == null) throw new Error(`no property info for: ${property} ${propKey}`);
      return { prop, propKey, isSubnode: true };
    } else {
      throw new Error(`no property info for: ${property}`);
    }
  }

  /** Editable property, subproperty or nested property row */
  function rowProperty(
    path: number | [number] | [number, number],
    options?: { title?: string; isFullWidth?: boolean; isDisabled?: boolean; default?: any },
  ): InspectViewRow {
    if (typeof path == "number") path = [path];

    const { prop: rootProp, propKey: rootPropKey, isSubnode } = property(path[0]);
    let prop: PropertyInfo;
    let propKeys: string[];
    if (path.length == 1) {
      prop = rootProp;
      propKeys = [rootPropKey];
    } else if (path.length == 2) {
      const rootPropType = getPropertyType(rootProp);
      const rootPropTypeInfos = PROPERTY_INFOS_BY_TYPE[rootPropType?.benchType as unknown as ObjectType];
      const rootPropEnum = PROPERTY_ENUM_BY_TYPE[rootPropType?.benchType as unknown as ObjectType];
      prop = rootPropTypeInfos?.[path[1]];
      const propKey = rootPropEnum?.[path[1]];
      if (prop == null || propKey == null) throw new Error(`no nested object at: ${path.join(".")}`);
      propKeys = [rootPropKey, propKey];
    } else {
      assertNever(path);
    }

    const title = options?.title ?? getPropertyTitle(prop);
    const propType = getPropertyType(prop);
    const view = getView(propType);
    if (view == null) throw new Error(`no view for property type: ${title}`);
    const row: InspectViewRow = {
      type: "view",
      title,
      isFullWidth: options?.isFullWidth || FULL_WIDTH_VIEW_TYPES.includes(view.type!),
      viewType: view.type!,
      viewProps: { ...view, isInput: !options?.isDisabled },
      read: () => {
        let val;
        if (path.length == 1) {
          val = (node as any)[rootPropKey];
        } else if (path.length == 2) {
          val = (node as any)[rootPropKey]?.[propKeys[1]];
        } else {
          assertNever(path);
        }
        return val ?? options?.default ?? prop.default;
      },
      write: (value) => {
        const tx = txFactory();
        const options: TransactionOptions = { debounce: "short" };
        if (path.length == 1) {
          if (!isSubnode) {
            tx.update(node, { [rootPropKey]: value }, options);
          } else {
            tx.update(
              node,
              // @ts-expect-error this is fine, metatype/subtype can't be typed properly here
              makeEditFromSubnode(node, { metatype, type: subtype, subnode: { [rootPropKey]: value } }),
              options,
            );
          }
        } else if (path.length == 2) {
          if (!isSubnode) {
            const newRootValue = makeStruct({
              metatype: rootProp.referenceStruct,
              ...(node as any)[rootPropKey],
              [propKeys[1]]: value,
            });
            tx.update(node, { [rootPropKey]: newRootValue }, options);
          } else {
            throw new Error(`nested subnode property edit not yet implemented`);
          }
        } else {
          assertNever(path);
        }
      },
    };
    return row;
  }

  /** Type row */
  function rowType(): InspectViewRow {
    const row: InspectViewRow = {
      type: "view",
      title: "Type",
      viewType: ViewType.PICKER,
      isFullWidth: false,
      viewProps: { valueType: makeTypeInfo({ benchType: BenchType.TYPE_INFO, isRequired: true }), isInput: true },
      read: () => node,
      write: (newType) => {
        updateFieldType(txFactory(), graph, node as FieldData, newType);
      },
    };
    return row;
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
      rows.push(rowProperty(ActionBlockProperty.toolsPtr, { isFullWidth: true }));
    }
    section("Action", rows, {
      summary: toCamelName(ActionMode, mode),
    });
  }

  function sectionRun(runOptionsProperty: number) {
    section(
      "Run",
      [
        rowProperty([runOptionsProperty, RunOptionsProperty.maxAttempts], { title: "Attempts" }),
        rowProperty([runOptionsProperty, RunOptionsProperty.maxRuns], { title: "Runs" }),
        rowProperty([runOptionsProperty, RunOptionsProperty.suppressAbort]),
        rowProperty([runOptionsProperty, RunOptionsProperty.suppressFail]),
      ],
      { isDefaultCollapsed: true },
    );
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
      sectionRun(BlockProperty.runOptions);
    }
  }

  //
  // Fields
  //
  else if (isNode(node, NodeType.FIELD)) {
    const commonRows: InspectRow[] = [rowProperty(FieldProperty.text)];
    if (node.type == FieldType.OPTION) {
      // commonRows.push() // color?
    } else {
      commonRows.push(rowType());
      if (node.type != FieldType.VARIABLE) {
        commonRows.push(rowProperty(FieldProperty.isRequired));
        commonRows.push(rowProperty(FieldProperty.isList));
      }
    }
    section(undefined, commonRows);
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
    sectionRun(StepProperty.runOptions);
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
    sectionRun(PipeProperty.runOptions);
  }

  //
  // Records
  //
  else if (isNode(node, NodeType.RECORD)) {
    section(undefined, [
      {
        type: "view",
        viewType: ViewType.OBJECT,
        viewProps: {
          variant: Variant.STEALTH,
          valueType: makeTypeInfo({
            kind: TypeKind.OBJECT,
            baseFieldType: FieldType.MEMBER,
            baseTypePtr: node.blockPtr,
          }),
          isInput: true,
          isInline: true,
        },
        isFullWidth: true,
        read: () => node.valuePacked,
        write: (value) => {},
      },
    ]);
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

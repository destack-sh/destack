import {
  BOUNDARY_ACTION_TYPES,
  getPropertyTitle,
  isNodeType,
  RUNNABLE_BLOCK_TYPES,
  SOURCE_NODE_TYPES,
  toCamelName,
} from "@/language/const";
import {
  createField,
  getPropertyType,
  makeTypeInfo,
  TypeIdentity,
  typeIsNumeric,
  updateFieldType,
} from "@/language/field";
import { ReadNodeGraph } from "@/language/graph";
import { unpackSubnode } from "@/language/node";
import {
  getTransactionOptionsForType,
  makeEditFromSubnode,
  Transaction,
  TransactionOptions,
} from "@/language/transaction";
import { packValue, unpackValue } from "@/language/value";
import {
  AnyNodeData,
  BenchType,
  BlockData,
  BlockProperty,
  BlockType,
  EditOperationData,
  EditOperationType,
  FailActionProperty,
  FieldData,
  FieldProperty,
  FieldType,
  IconData,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PipeProperty,
  PrimitiveType,
  PROPERTY_ENUM_BY_SUBTYPE,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_SUBTYPE,
  PROPERTY_INFOS_BY_TYPE,
  PropertyInfo,
  RecordProperty,
  RunOptionsProperty,
  ActionProperty,
  ActionType,
  TypeConstraintProperty,
  TypeKind,
  ViewType,
  ActionData,
} from "@/proto/wire";
import { isNode, makeStruct } from "@/proto/wiring";
import { canvas } from "@/system/globals";
import { ICON_BY_FIELD_TYPE, makeIcon } from "@/ui/icon";
import { pushPopover } from "@/ui/popover";
import { FULL_WIDTH_VIEW_TYPES, getViewForType } from "@/ui/view";
import { assertNever } from "@/utils/functools";
import { ModelValueOptions, ViewProps } from "@/views/common";

export type DetailLayout = {
  sections: DetailSection[];
};

export type DetailAction = {
  title: string;
  icon: IconData;
  action: (e: MouseEvent) => void;
};
export type DetailSection = {
  title?: string;
  subtitle?: string;
  rows: DetailRow[];
  isDefaultCollapsed?: boolean;
  summary?: string;
  actions?: DetailAction[];
};

type DetailRowBase = {
  title?: string;
  subtitle?: string;
};
export type DetailFieldsRow = DetailRowBase & {
  type: "fields";
  fieldType: FieldType;
  delegatePtr?: NodeReferenceData;
};
export type DetailViewRow = DetailRowBase & {
  type: "view";
  isFullWidth: boolean;
  viewType: ViewType;
  viewProps: ViewProps;
  read: () => any;
  write: (value: any, path?: any) => void;
};
export type DetailIconRow = DetailRowBase & {
  type: "icon";
  icon: IconData;
};
export type DetailTextRow = DetailRowBase & {
  type: "text";
  text: string;
};
export type DetailRow = DetailFieldsRow | DetailViewRow | DetailIconRow | DetailTextRow;

export function makeInspectLayout(node: AnyNodeData, graph: ReadNodeGraph, txFactory: () => Transaction): DetailLayout {
  const sections: DetailSection[] = [];

  const metatype = node.metatype as unknown as NodeType;
  const propertyEnum = PROPERTY_ENUM_BY_TYPE[node.metatype]!;
  const propertyInfos = PROPERTY_INFOS_BY_TYPE[node.metatype]!;
  const subtype = (node as any).type;
  const subpropertyEnum = subtype != null ? PROPERTY_ENUM_BY_SUBTYPE[metatype]?.[subtype] : undefined;
  const subpropertyInfos = PROPERTY_INFOS_BY_SUBTYPE[metatype]?.[subtype];
  const subnode =
    (node.subnodePacked as any)?.[subtype?.toString()] != null
      ? (unpackSubnode(metatype, subtype as never, node.subnodePacked) as any)
      : null;

  /** Make a Section */
  function section(
    title: string | undefined,
    rows: DetailRow[],
    options?: { isDefaultCollapsed?: boolean; subtitle?: string; summary?: string; actions?: DetailAction[] },
  ) {
    sections.push({
      title,
      subtitle: options?.subtitle,
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
    options?: {
      title?: string | false;
      subtitle?: string;
      isFullWidth?: boolean;
      isDisabled?: boolean;
      default?: any;
      props?: Partial<ViewProps>;
    },
  ): DetailRow {
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
    const propType = options?.props?.valueType ?? getPropertyType(prop);
    const view = getViewForType(propType);
    if (view == null) {
      return { type: "text", title: title === false ? undefined : title, subtitle: options?.subtitle, text: "No View" };
    }
    const row: DetailViewRow = {
      type: "view",
      title: title === false ? undefined : title,
      subtitle: options?.subtitle,
      isFullWidth: options?.isFullWidth || FULL_WIDTH_VIEW_TYPES.includes(view.type!),
      viewType: view.type!,
      viewProps: { ...view, ...options?.props, isInput: !options?.isDisabled },
      read: () => {
        let val;
        if (path.length == 1) {
          if (!isSubnode) {
            val = (node as any)[rootPropKey];
          } else {
            val = (subnode as any)?.[rootPropKey];
          }
        } else if (path.length == 2) {
          if (!isSubnode) {
            val = (node as any)[rootPropKey]?.[propKeys[1]];
          } else {
            val = (subnode as any)[rootPropKey]?.[propKeys[1]];
          }
        } else {
          assertNever(path);
        }
        return val ?? options?.default ?? prop.default;
      },
      write: (newValue) => {
        const tx = txFactory();
        const options: TransactionOptions = getTransactionOptionsForType(propType);
        if (path.length == 1) {
          if (!isSubnode) {
            tx.update(node, { [rootPropKey]: newValue }, options);
          } else {
            tx.update(
              node,
              // @ts-expect-error this is fine, metatype/subtype can't be typed properly here
              makeEditFromSubnode(node, { metatype, type: subtype, subnode: { [rootPropKey]: newValue } }),
              options,
            );
          }
        } else if (path.length == 2) {
          if (!isSubnode) {
            const newRootValue = makeStruct({
              metatype: rootProp.referenceStruct,
              ...(node as any)[rootPropKey],
              [propKeys[1]]: newValue,
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
  function rowType(): DetailViewRow {
    const row: DetailViewRow = {
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

  function actionAddField(
    fieldType: FieldType,
    icon: IconData | string = "fas fa-plus",
    options?: { delegatePtr?: NodeReferenceData },
  ): DetailAction {
    return {
      title: `Add ${toCamelName(FieldType, fieldType)}`,
      icon: makeIcon(icon),
      action: (e) => {
        const delegate = options?.delegatePtr != null ? graph.get(options.delegatePtr) : null;
        onAddFieldAction(e, fieldType, (delegate ?? node) as BlockData, graph, txFactory);
      },
    };
  }

  function sectionSchema(options?: { subtitle?: string; delegatePtr?: NodeReferenceData }) {
    section(
      "Schema",
      [
        { type: "fields", fieldType: FieldType.INPUT, delegatePtr: options?.delegatePtr },
        { type: "icon", icon: makeIcon("fas fa-arrow-down") },
        { type: "fields", fieldType: FieldType.OUTPUT, delegatePtr: options?.delegatePtr },
      ],
      {
        actions: [
          actionAddField(FieldType.INPUT, ICON_BY_FIELD_TYPE[FieldType.INPUT], options),
          actionAddField(FieldType.OUTPUT, ICON_BY_FIELD_TYPE[FieldType.OUTPUT], options),
        ],
        subtitle: options?.subtitle,
      },
    );
  }

  function sectionSchemaInput(options?: { subtitle?: string; delegatePtr?: NodeReferenceData }) {
    section("Input", [{ type: "fields", fieldType: FieldType.INPUT, delegatePtr: options?.delegatePtr }], {
      actions: [actionAddField(FieldType.INPUT, ICON_BY_FIELD_TYPE[FieldType.INPUT], options)],
      subtitle: options?.subtitle,
    });
  }

  function sectionSchemaOutput(options?: { subtitle?: string; delegatePtr?: NodeReferenceData }) {
    section("Output", [{ type: "fields", fieldType: FieldType.OUTPUT, delegatePtr: options?.delegatePtr }], {
      actions: [actionAddField(FieldType.OUTPUT, ICON_BY_FIELD_TYPE[FieldType.OUTPUT], options)],
      subtitle: options?.subtitle,
    });
  }

  function sectionRun(runOptionsProperty: number) {
    section(
      "Run",
      [
        rowProperty([runOptionsProperty, RunOptionsProperty.maxAttempts], { title: "Attempts" }),
        rowProperty([runOptionsProperty, RunOptionsProperty.suppressPause]),
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
    const commonRows: DetailRow[] = [];
    section(undefined, commonRows);
    if (node.type != BlockType.TEXT) {
      commonRows.push(rowProperty(BlockProperty.text, { title: false, props: { placeholder: "Text..." } }));
    }

    if (node.type == BlockType.CHOICE) {
      section("Options", [{ type: "fields", fieldType: FieldType.OPTION }], {
        actions: [actionAddField(FieldType.OPTION)],
      });
    } else if (node.type == BlockType.DATABASE || node.type == BlockType.MESSAGE) {
      section("Members", [{ type: "fields", fieldType: FieldType.MEMBER }], {
        actions: [actionAddField(FieldType.MEMBER)],
      });
    } else if (RUNNABLE_BLOCK_TYPES.includes(node.type)) {
      section("Variables", [{ type: "fields", fieldType: FieldType.VARIABLE }], {
        actions: [actionAddField(FieldType.VARIABLE)],
      });
      sectionSchema();
    }
    if (RUNNABLE_BLOCK_TYPES.includes(node.type)) {
      sectionRun(BlockProperty.runOptions);
    }
  }

  //
  // Fields
  //
  else if (isNode(node, NodeType.FIELD)) {
    const commonRows: DetailRow[] = [
      rowProperty(FieldProperty.text, { title: false, props: { placeholder: "Text..." } }),
    ];
    section(undefined, commonRows);
    if (node.type == FieldType.OPTION) {
      // color?
    } else {
      commonRows.push(rowType());
      if (node.type != FieldType.VARIABLE) {
        const base = node.baseTypePtr != null ? graph.get(node.baseTypePtr) : null;
        commonRows.push(rowProperty(FieldProperty.isRequired, { title: "Required" }));
        if (!(isNode(base, NodeType.BLOCK) && base.type == BlockType.DATABASE)) {
          // no lists for database fields yet :ManyToManyRecords
          commonRows.push(rowProperty(FieldProperty.isList, { title: "List" }));
        }
      }

      // default
      const defaultView = getViewForType(node, { forcePickerDropdown: true });
      if (defaultView?.type != null) {
        commonRows.push({
          type: "view",
          title: "Default",
          isFullWidth: FULL_WIDTH_VIEW_TYPES.includes(defaultView.type!),
          viewType: defaultView.type,
          viewProps: { ...defaultView, isInput: true },
          read() {
            if (node.defaultPacked == null) return null;
            const defaultUnpacked = unpackValue(node.defaultPacked, node, { graph, wrapScalar: true });
            return defaultUnpacked;
          },
          write: (newValue) => {
            txFactory().update(
              node,
              { defaultPacked: packValue(newValue, node, { graph, wrapScalar: true }) },
              getTransactionOptionsForType(node),
            );
          },
        });
      }

      const constraintRows: DetailRow[] = [];
      // list
      if (node.isList || node.primitiveType == PrimitiveType.STRING) {
        constraintRows.push(
          rowProperty([FieldProperty.constraint, TypeConstraintProperty.minLength], { title: "Minimum Length" }),
        );
        constraintRows.push(
          rowProperty([FieldProperty.constraint, TypeConstraintProperty.maxLength], { title: "Maximum Length" }),
        );
      }
      // stringy
      if (node.primitiveType == PrimitiveType.STRING) {
        constraintRows.push(
          rowProperty([FieldProperty.constraint, TypeConstraintProperty.startsWith], { title: "Prefix" }),
        );
        constraintRows.push(
          rowProperty([FieldProperty.constraint, TypeConstraintProperty.endsWith], { title: "Suffix" }),
        );
        constraintRows.push(rowProperty([FieldProperty.constraint, TypeConstraintProperty.regex], { title: "Regex" }));
      }
      // number
      if (typeIsNumeric(node)) {
        constraintRows.push(
          rowProperty([FieldProperty.constraint, TypeConstraintProperty.minValue], { title: "Minimum" }),
        );
        constraintRows.push(
          rowProperty([FieldProperty.constraint, TypeConstraintProperty.maxValue], { title: "Maximum" }),
        );
        constraintRows.push(
          rowProperty([FieldProperty.constraint, TypeConstraintProperty.stepValue], { title: "Step" }),
        );
      }
      // node
      if (isNodeType(node.benchType) && SOURCE_NODE_TYPES.includes(node.benchType)) {
        const nodeProperties = PROPERTY_INFOS_BY_TYPE[node.benchType as unknown as NodeType];
        const nodePropertiesEnum = PROPERTY_ENUM_BY_TYPE[node.benchType as unknown as NodeType];
        const subtypeProperty = nodeProperties[(nodePropertiesEnum as any)?.["type"]!];
        if (subtypeProperty?.enumType != null) {
          constraintRows.push(
            rowProperty([FieldProperty.constraint, TypeConstraintProperty.nodeSubtypes], {
              title: `${toCamelName(BenchType, node.benchType)} Type`,
              props: {
                valueType: makeTypeInfo({
                  kind: TypeKind.ENUM,
                  benchType: subtypeProperty.enumType as unknown as BenchType,
                  isList: true,
                }),
              },
            }),
          );
        }
        constraintRows.push(
          rowProperty([FieldProperty.constraint, TypeConstraintProperty.nodeScopePtr], {
            title: "Scope",
            props: {
              valueType: makeTypeInfo({ kind: TypeKind.NODE, benchType: BenchType.BLOCK, isList: true }),
            },
          }),
        );
      }

      if (constraintRows.length > 0) {
        section("Constraint", constraintRows, { isDefaultCollapsed: true });
      }
    }
  }

  //
  // Actions
  //
  else if (isNode(node, NodeType.ACTION)) {
    const commonRows: DetailRow[] = [];
    section(undefined, commonRows);
    commonRows.push(rowProperty(ActionProperty.text, { title: false, props: { placeholder: "Text..." } }));

    if (node.type == ActionType.CODE) {
      commonRows.push(rowProperty(ActionProperty.code, { isFullWidth: true }));
    } else if (node.type == ActionType.DELEGATE) {
      commonRows.push(rowProperty(ActionProperty.delegatePtr, { isFullWidth: false }));
      const action = subnode as ActionData | undefined;
      // schema from delegate
      const delegatePtr = action?.delegatePtr;
      if (delegatePtr != null) {
        sectionSchema({ subtitle: "(Delegate)", delegatePtr });
      } else {
        section("Schema", [{ type: "text", text: "No delegate set." }]);
      }
    } else if (node.type == ActionType.FAIL) {
      section("Error", [
        rowProperty(FailActionProperty.errorTitle, { title: "Title" }),
        rowProperty(FailActionProperty.errorText, { title: "Text" }),
      ]);
    } else {
      // add all from subproperty enum
      if (subpropertyEnum != null) {
        Object.values(subpropertyEnum)
          .filter((v) => typeof v == "number")
          .forEach((subproperty) => {
            commonRows.push(rowProperty(subproperty as any));
          });
      }
    }

    if (!BOUNDARY_ACTION_TYPES.includes(node.type)) {
      sectionRun(ActionProperty.runOptions);
    }
  }

  //
  // Pipes
  //
  else if (isNode(node, NodeType.PIPE)) {
    section(undefined, [
      rowProperty(PipeProperty.text, { title: false, props: { placeholder: "Text..." } }),
      rowProperty(PipeProperty.type),
      rowProperty(PipeProperty.color),
      rowProperty(PipeProperty.delay),
    ]);
    sectionRun(PipeProperty.runOptions);
  }

  //
  // Records
  //
  else if (isNode(node, NodeType.RECORD)) {
    section(undefined, [
      rowProperty(RecordProperty.text, { title: false, props: { placeholder: "Text..." } }),
      {
        type: "view",
        viewType: ViewType.OBJECT,
        viewProps: {
          valueType: makeTypeInfo({
            kind: TypeKind.CUSTOM_OBJECT,
            baseFieldType: FieldType.MEMBER,
            baseTypePtr: node.blockPtr,
          }),
          isInput: true,
          isInline: true,
          isMinimal: true,
        },
        isFullWidth: true,
        read: () => node.valuePacked,
        write: (newValue, options?: ModelValueOptions) => {
          if (options == null) {
            txFactory().update(node, { valuePacked: newValue });
          } else {
            const operations: EditOperationData[] = [
              {
                metatype: ObjectType.EDIT_OPERATION,
                type: newValue == null ? EditOperationType.CLEAR : EditOperationType.SET,
                path: [RecordProperty.valuePacked.toString(), ...options.path],
                newValuePacked: (newValue as any)?.[options.path[0]],
                oldValuePacked: (node.valuePacked as any)?.[options.path[0]],
              },
            ];
            txFactory().update(node, operations, getTransactionOptionsForType(options.field));
          }
        },
      },
    ]);
  }

  const layout: DetailLayout = { sections };
  return layout;
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
    const field = createField(txFactory(), graph, {
      anchor: "inside",
      target: parent!,
      field: { type: fieldType },
    });
    canvas.inspect({ node: field });
  } else {
    const button = (e.target as HTMLElement).closest("button")!;
    pushPopover({
      kind: "view",
      trigger: button,
      reference: button,
      title: `Add ${toCamelName(FieldType, fieldType)}`,
      component: ViewType.PICKER,
      placement: "bottom-left",
      offset: "referenceWidth",
      props: { valueType: makeTypeInfo({ benchType: BenchType.TYPE_INFO }) },
      onApply: (typeInfo: TypeIdentity) => {
        if (fieldType == FieldType.VARIABLE) {
          typeInfo = { ...typeInfo, isRequired: true }; // variables are required by default
        }
        const field = createField(txFactory(), graph, {
          anchor: "inside",
          target: parent,
          field: { ...typeInfo, type: fieldType },
        });
        canvas.inspect({ node: field });
      },
    });
  }
}

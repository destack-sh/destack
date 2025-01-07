import {
  BOUNDARY_ACTION_TYPES,
  getPropertyTitle,
  isNodeType,
  isSourceNode,
  RUNNABLE_BLOCK_TYPES,
  SOURCE_NODE_TYPES,
  toCamelName,
} from "@/language/const";
import {
  createField,
  getPropertyType,
  getTypeName,
  makeTypeInfo,
  TypeIdentity,
  typeIsNumeric,
  updateFieldType,
} from "@/language/field";
import { ReadNodeGraph } from "@/language/graph";
import { packSubnode, unpackSubnode } from "@/language/node";
import { getPathKey, makePath } from "@/language/path";
import {
  getTransactionOptionsForType,
  makeEditFromSubnode,
  newChangeId,
  Transaction,
  TransactionOptions,
} from "@/language/transaction";
import { packValue, unpackValue } from "@/language/value";
import {
  ActionData,
  ActionProperty,
  ActionType,
  AnyNodeData,
  BenchType,
  BlockData,
  BlockProperty,
  BlockType,
  ComputedValueData,
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
  PathData,
  PathElementType,
  PickerVariant,
  PipeProperty,
  PrimitiveType,
  PROPERTY_ENUM_BY_SUBTYPE,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_SUBTYPE,
  PROPERTY_INFOS_BY_TYPE,
  PropertyInfo,
  RecordProperty,
  RunOptionsProperty,
  RunProperty,
  TypeConstraintProperty,
  TypeKind,
  ViewType,
} from "@/proto/wire";
import { isNode, makeStruct, propertyReference, toNodeRef, toPropertyRef } from "@/proto/wiring";
import { canvas, supergraph } from "@/system/globals";
import { getNodeName, ICON_BY_FIELD_TYPE, makeIcon } from "@/ui/icon";
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
  key?: string;
  title?: string;
  subtitle?: string;
  rows: DetailRow[];
  isDefaultCollapsed?: boolean;
  summary?: string;
  actions?: DetailAction[];
};

type RowBase = {
  title?: string;
  subtitle?: string;
};
export type FieldsRow = RowBase & {
  type: "fields";
  fieldType: FieldType;
  toolPtr?: NodeReferenceData;
};
export type ViewRow = RowBase & {
  type: "view";
  isFullWidth: boolean;
  viewType: ViewType;
  viewProps: ViewProps;
  read: () => any;
  write: (value: any, path?: any) => void;
};
export type ObjectRow = Omit<ViewRow, "type"> & {
  type: "object";
  isComputable: boolean;
  computedPath?: PathData;
  computedPathKey?: string;
  prop: PropertyInfo;
};
export type PropertyRow = Omit<ViewRow, "type"> & {
  type: "property";
  isComputable: boolean;
  computedPath?: PathData;
  computedPathKey?: string;
  computedValue?: ComputedValueData;
  prop: PropertyInfo;
};
export type IconRow = RowBase & {
  type: "icon";
  icon: IconData;
};
export type LineRow = RowBase & {
  type: "line";
  text?: string;
};
export type TextRow = RowBase & {
  type: "text";
  text: string;
};
export type DetailRow = FieldsRow | ViewRow | PropertyRow | ObjectRow | IconRow | TextRow | LineRow;

export function makeDetailLayout(node: AnyNodeData, graph: ReadNodeGraph, txFactory: () => Transaction): DetailLayout {
  // nocheckin: edit ComputedValues in Detail layout (incl. in CustomObject for variables/inputs, node partials, ... Path view?)

  // node stuff
  const nodePtr = toNodeRef(node);
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
  const computedValues = isSourceNode(node) ? node.computedValues : undefined;
  const computedValuesByKey: Record<string, ComputedValueData> = (computedValues ?? [])
    .filter((cv) => cv.targetPath != null)
    .reduce(
      (acc, cv) => {
        acc[getPathKey(cv.targetPath!)] = cv;
        return acc;
      },
      {} as Record<string, ComputedValueData>,
    );

  /** Make a Section */
  const sections: DetailSection[] = [];
  function section(
    title: string | undefined,
    rows: DetailRow[],
    options?: {
      key?: string;
      isDefaultCollapsed?: boolean;
      subtitle?: string;
      summary?: string;
      actions?: DetailAction[];
    },
  ): DetailSection {
    const s: DetailSection = {
      key: options?.key ?? (title != null ? `section-${title}-${options?.subtitle ?? ""}` : undefined),
      title,
      subtitle: options?.subtitle,
      rows,
      isDefaultCollapsed: options?.isDefaultCollapsed,
      summary: options?.summary,
      actions: options?.actions,
    };
    sections.push(s);
    return s;
  }

  /** Gets a top-level property with the given key */
  function property(property: number): { prop: PropertyInfo; propName: string; isSubnode: boolean } {
    let prop: PropertyInfo;
    let propName: string;
    if (propertyInfos[property] != null) {
      prop = propertyInfos[property];
      propName = propertyEnum[property];
      if (prop == null) throw new Error(`no property info for: ${property}`);
      return { prop, propName, isSubnode: false };
    } else if (subpropertyInfos?.[property] != null) {
      prop = subpropertyInfos[property];
      propName = subpropertyEnum![property];
      if (prop == null) throw new Error(`no property info for: ${property} ${propName}`);
      return { prop, propName, isSubnode: true };
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
      isComputable?: boolean;
      default?: any;
      props?: Partial<ViewProps>;
      extendWrite?: (tx: Transaction, newValue: any, options: TransactionOptions) => void;
    },
  ): DetailRow {
    if (typeof path == "number") path = [path];

    // property
    const { prop: rootProp, propName: rootPropName, isSubnode } = property(path[0]);
    let prop: PropertyInfo;
    let propNames: string[];
    if (path.length == 1) {
      prop = rootProp;
      propNames = [rootPropName];
    } else if (path.length == 2) {
      const rootPropType = getPropertyType(rootProp);
      const rootPropTypeInfos = PROPERTY_INFOS_BY_TYPE[rootPropType?.benchType as unknown as ObjectType];
      const rootPropEnum = PROPERTY_ENUM_BY_TYPE[rootPropType?.benchType as unknown as ObjectType];
      prop = rootPropTypeInfos?.[path[1]];
      const propName = rootPropEnum?.[path[1]];
      if (prop == null || propName == null) throw new Error(`no nested object at: ${path.join(".")}`);
      propNames = [rootPropName, propName];
    } else {
      assertNever(path);
    }

    // computed
    // NOTE :Incomplete: currently computed values UI is focused on :RunComputed values
    //  (as opposed to templated ones in non-runnable Nodes, i.e. we're ignoring non-runtime computation contexts)
    let computedPath: PathData | undefined = undefined;
    let computedPathKey: string | undefined = undefined;
    if (options?.isComputable) {
      computedPath = makePath(
        PathElementType.RUN,
        propertyReference(NodeType.RUN, RunProperty.inputsPacked),
        toPropertyRef(prop),
      );
      computedPathKey = getPathKey(computedPath);
    }
    const computedValue = computedPathKey != null ? computedValuesByKey[computedPathKey] : undefined;

    // view
    const title = options?.title ?? getPropertyTitle(prop);
    const propType = options?.props?.valueType ?? getPropertyType(prop);
    const view = getViewForType(propType, { forcePickerDropdown: true });
    if (view == null) {
      return { type: "text", title: title === false ? undefined : title, subtitle: options?.subtitle, text: "No View" };
    }
    const row: PropertyRow = {
      type: "property",
      prop,
      title: title === false ? undefined : title,
      subtitle: options?.subtitle,
      isComputable: options?.isComputable ?? false,
      computedPath,
      computedPathKey,
      computedValue,
      isFullWidth: options?.isFullWidth || computedValue?.isActive || FULL_WIDTH_VIEW_TYPES.includes(view.type!),
      viewType: view.type!,
      viewProps: { ...view, ...options?.props, isInput: !options?.isDisabled },
      read: () => {
        let val;
        if (path.length == 1) {
          if (!isSubnode) {
            val = (node as any)[rootPropName];
          } else {
            val = (subnode as any)?.[rootPropName];
          }
        } else if (path.length == 2) {
          if (!isSubnode) {
            val = (node as any)[rootPropName]?.[propNames[1]];
          } else {
            val = (subnode as any)[rootPropName]?.[propNames[1]];
          }
        } else {
          assertNever(path);
        }
        return val ?? options?.default ?? prop.default;
      },
      write: (newValue) => {
        let tx = txFactory();
        const txOptions: TransactionOptions = getTransactionOptionsForType(propType);
        if (options?.extendWrite != null) {
          if (tx.change?.key == null) {
            tx = tx.with({ change: { key: newChangeId() } });
          }
        }
        if (path.length == 1) {
          if (!isSubnode) {
            tx.update(node, { [rootPropName]: newValue }, txOptions);
          } else {
            tx.update(
              node,
              // @ts-expect-error this is fine, metatype/subtype can't be typed properly here
              makeEditFromSubnode(node, { metatype, type: subtype, subnode: { [rootPropName]: newValue } }),
              txOptions,
            );
          }
        } else if (path.length == 2) {
          if (!isSubnode) {
            const newRootValue = makeStruct({
              metatype: rootProp.referenceStruct,
              ...(node as any)[rootPropName],
              [propNames[1]]: newValue,
            });
            tx.update(node, { [rootPropName]: newRootValue }, txOptions);
          } else {
            throw new Error(`nested subnode property edit not yet implemented`);
          }
        } else {
          assertNever(path);
        }
        if (options?.extendWrite != null) {
          options.extendWrite(tx, newValue, txOptions);
        }
      },
    };
    return row;
  }

  /** Type row */
  function rowType(options?: { title?: string; extendWrite?: (tx: Transaction, newValue: any) => void }): ViewRow {
    const row: ViewRow = {
      type: "view",
      title: options?.title ?? "Type",
      viewType: ViewType.TYPE,
      isFullWidth: false,
      viewProps: { valueType: makeTypeInfo({ benchType: BenchType.TYPE_INFO, isRequired: true }), isInput: true },
      read: () => node,
      write: (newType) => {
        let tx = txFactory();
        if (options?.extendWrite != null) {
          if (tx.change?.key == null) {
            tx = tx.with({ change: { key: newChangeId() } });
          }
        }
        updateFieldType(tx, graph, node as FieldData, newType);
        if (options?.extendWrite != null) {
          options.extendWrite(tx, newType);
        }
      },
    };
    return row;
  }

  function actionAddField(
    fieldType: FieldType,
    icon: IconData | string = "fas fa-plus",
    options?: { toolPtr?: NodeReferenceData },
  ): DetailAction {
    return {
      title: `Add ${toCamelName(FieldType, fieldType)}`,
      icon: makeIcon(icon),
      action: (e) => {
        const tool = options?.toolPtr != null ? graph.get(options.toolPtr) : null;
        onAddFieldAction(e, fieldType, (tool ?? node) as BlockData, graph, txFactory, { dontFocus: true });
      },
    };
  }

  /** Object row */
  function rowObject(
    propertyId: number,
    valueType: TypeIdentity,
    options?: { title?: string | false; subtitle?: string; isComputable?: boolean },
  ): ObjectRow {
    const { prop, propName } = property(propertyId);

    // computed :RunComputed
    let computedPath: PathData | undefined = undefined;
    let computedPathKey: string | undefined = undefined;
    if (options?.isComputable) {
      computedPath = makePath(
        PathElementType.RUN,
        propertyReference(NodeType.RUN, RunProperty[propName as any as keyof typeof RunProperty]),
        toPropertyRef(prop),
      );
      computedPathKey = getPathKey(computedPath);
    }

    // view
    const title = options?.title ?? getPropertyTitle(prop);
    const row: ObjectRow = {
      type: "object",
      title: title === false ? undefined : title,
      subtitle: options?.subtitle,
      isComputable: options?.isComputable ?? false,
      prop,
      viewType: ViewType.OBJECT,
      computedPath,
      computedPathKey,
      viewProps: { valueType: makeTypeInfo(valueType), isInput: true, isInline: true, isMinimal: true },
      isFullWidth: true,
      read: () => (node as any)[propName],
      write: (newValue, options?: ModelValueOptions) => {
        if (options == null) {
          txFactory().update(node, { [propName]: newValue });
        } else {
          const operations: EditOperationData[] = [
            {
              metatype: ObjectType.EDIT_OPERATION,
              type: newValue == null ? EditOperationType.CLEAR : EditOperationType.SET,
              path: [propertyId.toString(), ...options.path],
              newValuePacked: (newValue as any)?.[options.path[0]],
              oldValuePacked: (node as any)?.[options.path[0]],
            },
          ];
          txFactory().update(node, operations, getTransactionOptionsForType(options.field));
        }
      },
    };
    return row;
  }

  /** Line row */
  function rowLine(text?: string): LineRow {
    return { type: "line", text };
  }

  /** Icon row */
  function rowIcon(icon: IconData | string): IconRow {
    return { type: "icon", icon: makeIcon(icon) };
  }

  function sectionSchema(options?: { title?: string; subtitle?: string; toolPtr?: NodeReferenceData }): DetailSection {
    return section(
      options?.title ?? "Schema",
      [
        { type: "fields", fieldType: FieldType.INPUT, toolPtr: options?.toolPtr },
        { type: "icon", icon: makeIcon("fas fa-arrow-down") },
        { type: "fields", fieldType: FieldType.OUTPUT, toolPtr: options?.toolPtr },
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

  function sectionRun(runOptionsProperty: number) {
    section(
      "Run",
      [
        rowProperty([runOptionsProperty, RunOptionsProperty.maxAttempts], { title: "Attempts" }),
        rowProperty([runOptionsProperty, RunOptionsProperty.suppressFail]),
        rowProperty([runOptionsProperty, RunOptionsProperty.modelFamily]),
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
      commonRows.push(
        rowType({
          extendWrite: (tx, newType) => {
            // also update field name if type changes
            const name = getTypeName(newType);
            if (name != null) {
              tx.update(node, { name });
            }
          },
        }),
      );

      // default value
      if (node.type == FieldType.VARIABLE || node.type == FieldType.MEMBER) {
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

    // common rows
    if (node.type == ActionType.CODE) {
      commonRows.push(rowProperty(ActionProperty.code, { isFullWidth: true, isComputable: true }));
    } else if (node.type == ActionType.TOOL) {
      commonRows.push(
        rowProperty(ActionProperty.toolPtr, {
          isComputable: true,
          isFullWidth: false,
          extendWrite: (tx, newValue, options) => {
            // also update node name if tool changes
            const tool = newValue != null ? supergraph.get(newValue) : null;
            const name = tool != null ? getNodeName(tool) : null;
            if (name != null) {
              tx.update(node, { name }, options);
            }
          },
        }),
      );
    } else if (node.type == ActionType.FAIL) {
      commonRows.push(rowProperty(FailActionProperty.errorTitle, { title: "Title", isComputable: true }));
      commonRows.push(rowProperty(FailActionProperty.errorText, { title: "Text", isComputable: true }));
    } else {
      // add all from subproperty enum
      if (subpropertyEnum != null) {
        Object.values(subpropertyEnum)
          .filter((v) => typeof v == "number")
          .forEach((subproperty) => {
            const { prop } = property(subproperty as any);
            if (prop != null && prop.fieldType != FieldType.OUTPUT) {
              commonRows.push(rowProperty(subproperty, { isComputable: true }));
            }
          });
      }
    }

    // schema
    let toolPtr: NodeReferenceData | undefined = undefined;
    if (node.type == ActionType.START || node.type == ActionType.COMPLETE) {
      toolPtr = node.parentPtr;
    } else if (node.type == ActionType.TOOL) {
      toolPtr = node.toolPtr;
    }
    if (node.type == ActionType.START) {
      // flow inputs
      section("Schema", [{ type: "fields", fieldType: FieldType.INPUT, toolPtr: node.parentPtr }], {
        subtitle: "(Flow)",
        actions: [actionAddField(FieldType.INPUT, ICON_BY_FIELD_TYPE[FieldType.INPUT], { toolPtr: node.parentPtr })],
      });
    } else if (node.type == ActionType.COMPLETE) {
      // ƒlow outputs as inputs
      section(
        "Schema",
        [
          rowObject(
            ActionProperty.inputsPacked,
            makeTypeInfo({
              kind: TypeKind.CUSTOM_OBJECT,
              baseFieldType: FieldType.OUTPUT,
              baseTypePtr: node.parentPtr,
            }),
            { title: false, isComputable: true },
          ),
        ],
        {
          subtitle: "(Flow)",
          actions: [
            actionAddField(FieldType.OUTPUT, ICON_BY_FIELD_TYPE[FieldType.OUTPUT], { toolPtr: node.parentPtr }),
          ],
        },
      );
    } else if (node.type == ActionType.TOOL && toolPtr != null) {
      // own schema with tool schema
      section(
        "Schema",
        [
          // tool variables & inputs
          rowObject(
            ActionProperty.variablesPacked,
            makeTypeInfo({ kind: TypeKind.CUSTOM_OBJECT, baseFieldType: FieldType.VARIABLE, baseTypePtr: toolPtr }),
            { title: false, isComputable: true },
          ),
          rowObject(
            ActionProperty.inputsPacked,
            makeTypeInfo({ kind: TypeKind.CUSTOM_OBJECT, baseFieldType: FieldType.INPUT, baseTypePtr: toolPtr }),
            { title: false, isComputable: true },
          ),
          // arrow
          rowIcon("fas fa-arrow-down"),
          // outputs
          { type: "fields", fieldType: FieldType.OUTPUT, toolPtr },
          rowLine("Self"),
          { type: "fields", fieldType: FieldType.OUTPUT },
        ],
        {
          subtitle: "(Tool)",
          actions: [
            actionAddField(FieldType.INPUT, ICON_BY_FIELD_TYPE[FieldType.INPUT]),
            actionAddField(FieldType.OUTPUT, ICON_BY_FIELD_TYPE[FieldType.OUTPUT]),
          ],
        },
      );
    } else if (node.type != ActionType.FAIL) {
      // own schema
      section(
        "Schema",
        [
          rowObject(
            ActionProperty.inputsPacked,
            makeTypeInfo({
              kind: TypeKind.CUSTOM_OBJECT,
              baseFieldType: FieldType.INPUT,
              baseTypePtr: toolPtr ?? nodePtr,
            }),
            { title: false, isComputable: true },
          ),
          rowIcon("fas fa-arrow-down"),
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

    // run options
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
  }

  //
  // Records
  //
  else if (isNode(node, NodeType.RECORD)) {
    section(undefined, [rowProperty(RecordProperty.text, { title: false, props: { placeholder: "Text..." } })]);
    section(
      "Schema",
      [
        rowObject(
          RecordProperty.valuePacked,
          makeTypeInfo({ kind: TypeKind.CUSTOM_OBJECT, baseFieldType: FieldType.MEMBER, baseTypePtr: node.blockPtr }),
          { title: false },
        ),
      ],
      {
        actions: [actionAddField(FieldType.MEMBER)],
      },
    );
  }

  const layout: DetailLayout = { sections };
  return layout;
}

/** Handle an 'add Field' button (either directly or by spawning a Popover) */
export function onAddFieldAction(
  e: MouseEvent,
  fieldType: FieldType,
  parent: BlockData | ActionData,
  graph: ReadNodeGraph,
  txFactory: () => Transaction,
  options?: { dontFocus?: boolean },
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
      props: {
        valueType: makeTypeInfo({ benchType: BenchType.TYPE_INFO }),
        subnodePacked: packSubnode(NodeType.VIEW, ViewType.PICKER, { variant: PickerVariant.DROPDOWN_LARGE }),
      },
      onApply: (typeInfo: TypeIdentity) => {
        if (fieldType == FieldType.VARIABLE) {
          typeInfo = { ...typeInfo, isRequired: true }; // variables are required by default
        }
        const field = createField(txFactory(), graph, {
          anchor: "inside",
          target: parent,
          field: { ...typeInfo, type: fieldType },
        });
        if (!options?.dontFocus) {
          canvas.inspect({ node: field });
        }
      },
    });
  }
}

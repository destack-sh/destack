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
  getFieldTypeUpdate,
  getPropertyType,
  getTypeName,
  makeType,
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
import { getCustomObjectNodeType, getCustomObjectSubtype, packValue, unpackValue } from "@/language/value";
import {
  ActionData,
  ActionProperty,
  ActionType,
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
  TypeData,
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

export type ObjectAction = {
  title: string;
  icon: IconData;
  action: (e: MouseEvent) => void;
};
export type ObjectSection = {
  key?: string;
  title?: string;
  subtitle?: string;
  rows: DetailRow[];
  isDefaultCollapsed?: boolean;
  summary?: string;
  actions?: ObjectAction[];
};

type RowBase = {
  title?: string;
  subtitle?: string;
};
export type FieldsListRow = RowBase & {
  type: "fields-list";
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
  computedType?: TypeData;
  prop: PropertyInfo;
};
export type PropertyRow = Omit<ViewRow, "type"> & {
  type: "property";
  isComputable: boolean;
  computedPath?: PathData;
  computedPathKey?: string;
  prop: PropertyInfo;
};
export type FieldRow = Omit<ViewRow, "type"> & {
  type: "field";
  isComputable: boolean;
  computedPath?: PathData;
  computedPathKey?: string;
  field: FieldType;
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
export type DetailRow = FieldsListRow | ViewRow | PropertyRow | FieldRow | ObjectRow | IconRow | TextRow | LineRow;

/** Get the layout for a detailed view of a CustomObject or Node› */
export class ObjectLayout {
  kind: "node" | "partial" | "custom";

  // node
  private nodeType: NodeType | null;
  private subtype: number | null;
  private propertyEnum: Record<number, string>;
  private propertyInfos: Record<number, PropertyInfo>;
  private subpropertyEnum?: Record<number, string>;
  private subpropertyInfos?: Record<number, PropertyInfo>;
  private subnode: any;

  private node: AnyNodeData | null;
  private base: AnyNodeData | null;
  private baseFields: FieldData[];
  private graph: ReadNodeGraph;
  private update: (
    update: Partial<AnyNodeData> | EditOperationData[] | Record<string, any>,
    options?: TransactionOptions,
  ) => void;
  private txFactory: () => Transaction;
  private valuePacked?: Record<string, any>;
  private valueType?: TypeData;
  private computedType?: TypeData;

  // layout
  sections: ObjectSection[] = [];

  constructor(options: {
    kind: "node" | "partial" | "custom";
    node: AnyNodeData | null;
    base: AnyNodeData | null;
    baseFields: FieldData[];
    graph: ReadNodeGraph;
    update: (update: Partial<AnyNodeData> | Record<string, any>, options?: TransactionOptions) => void;
    txFactory: () => Transaction;
    valuePacked?: Record<string, any>;
    valueType?: TypeData;
    computedType?: TypeData;
  }) {
    this.kind = options.kind;
    this.node = options.node;
    this.base = options.base;
    this.baseFields = options.baseFields;
    this.graph = options.graph;
    this.update = options.update;
    this.txFactory = options.txFactory;
    this.valuePacked = options.valuePacked;
    this.valueType = options.valueType;
    this.computedType = options.computedType;

    // initialize object stuff
    if (this.kind == "node") {
      if (this.node == null) throw new Error("node is null");
      this.nodeType = this.node.metatype as unknown as NodeType;
      this.subtype = (this.node as any).type;
      this.subnode =
        (this.node.subnodePacked as any)?.[this.subtype?.toString()!] != null
          ? (unpackSubnode(this.nodeType, this.subtype as never, this.node.subnodePacked) as any)
          : null;
    } else if (this.kind == "partial") {
      if (this.valueType == null) throw new Error("valueType is null");
      this.nodeType = getCustomObjectNodeType(this.valueType, this.valuePacked ?? {});
      this.subtype = getCustomObjectSubtype(this.valueType, this.valuePacked ?? {});
    } else if (this.kind == "custom") {
      if (this.valueType == null) throw new Error("valueType is null");
      this.nodeType = null;
      this.subtype = null;
    } else {
      assertNever(this.kind);
    }
    this.propertyEnum = this.nodeType != null ? PROPERTY_ENUM_BY_TYPE[this.nodeType]! : {};
    this.propertyInfos = this.nodeType != null ? PROPERTY_INFOS_BY_TYPE[this.nodeType]! : {};
    this.subpropertyEnum = this.subtype != null ? PROPERTY_ENUM_BY_SUBTYPE[this.nodeType!]?.[this.subtype] : undefined;
    this.subpropertyInfos = this.subtype != null ? PROPERTY_INFOS_BY_SUBTYPE[this.nodeType!]?.[this.subtype] : {};
  }

  /** Add a section to the layout */
  section(
    title: string | undefined,
    rows: DetailRow[],
    options?: {
      key?: string;
      isDefaultCollapsed?: boolean;
      subtitle?: string;
      summary?: string;
      actions?: ObjectAction[];
    },
  ): ObjectSection {
    const s: ObjectSection = {
      key: options?.key ?? (title != null ? `section-${title}-${options?.subtitle ?? ""}` : undefined),
      title,
      subtitle: options?.subtitle,
      rows,
      isDefaultCollapsed: options?.isDefaultCollapsed,
      summary: options?.summary,
      actions: options?.actions,
    };
    this.sections.push(s);
    return s;
  }

  /** Get the property info for a property */
  getProperty(property: number): { prop: PropertyInfo; propName: string; isSubnode: boolean } {
    let prop: PropertyInfo;
    let propName: string;
    if (this.propertyInfos[property] != null) {
      prop = this.propertyInfos[property];
      propName = this.propertyEnum[property];
      if (prop == null) throw new Error(`no property info for: ${property}`);
      return { prop, propName, isSubnode: false };
    } else if (this.subpropertyInfos?.[property] != null) {
      prop = this.subpropertyInfos[property];
      propName = this.subpropertyEnum![property];
      if (prop == null) throw new Error(`no property info for: ${property} ${propName}`);
      return { prop, propName, isSubnode: true };
    } else {
      throw new Error(`no property info for: ${property}`);
    }
  }

  /** Get property path */
  getPropertyPath(path: number | [number] | [number, number]) {
    if (typeof path == "number") path = [path];
    const { prop: rootProp, propName: rootPropName, isSubnode } = this.getProperty(path[0]);
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
    return { path, prop, propNames, rootProp, rootPropName, isSubnode };
  }

  /** Read a property from a node or partial node */
  readProperty(pathIn: number | [number] | [number, number], options?: { default?: any }) {
    const { path, prop, propNames, rootProp, rootPropName, isSubnode } = this.getPropertyPath(pathIn);
    let val;
    if (path.length == 1) {
      if (!isSubnode) {
        val = (this.node as any)?.[rootPropName];
      } else {
        val = (this.subnode as any)?.[rootPropName];
      }
    } else if (path.length == 2) {
      if (!isSubnode) {
        val = (this.node as any)[rootPropName]?.[propNames[1]];
      } else {
        val = (this.subnode as any)[rootPropName]?.[propNames[1]];
      }
    } else {
      assertNever(path);
    }
    return val ?? options?.default ?? prop.default;
  }

  /** Property row for node or partial node */
  rowProperty(
    pathIn: number | [number] | [number, number],
    options?: {
      title?: string | false;
      subtitle?: string;
      isFullWidth?: boolean;
      isDisabled?: boolean;
      isComputable?: boolean;
      default?: any;
      props?: Partial<ViewProps>;
      extendUpdate?: (newValue: any, options: TransactionOptions) => Record<number, any>;
    },
  ): DetailRow {
    const { path, prop, propNames, rootProp, rootPropName, isSubnode } = this.getPropertyPath(pathIn);

    // computed
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
      isFullWidth: options?.isFullWidth || FULL_WIDTH_VIEW_TYPES.includes(view.type!),
      viewType: view.type!,
      viewProps: { ...view, ...options?.props, isInput: !options?.isDisabled },
      read: () => this.readProperty(pathIn, options),
      write: (newValue) => {
        if (this.kind == "node") {
          const txOptions: TransactionOptions = getTransactionOptionsForType(propType);
          let update: Record<string, any>;
          if (path.length == 1) {
            if (!isSubnode) {
              update = { [rootPropName]: newValue };
            } else {
              update = makeEditFromSubnode(this.node!, {
                metatype: this.nodeType as any,
                type: this.subtype,
                subnode: { [rootPropName]: newValue },
              });
            }
          } else if (path.length == 2) {
            if (!isSubnode) {
              const newRootValue = makeStruct({
                metatype: rootProp.referenceStruct,
                ...(this.node as any)[rootPropName],
                [propNames[1]]: newValue,
              });
              update = { [rootPropName]: newRootValue };
            } else {
              throw new Error(`nested subnode property edit not yet implemented`);
            }
          } else {
            assertNever(path);
          }

          if (options?.extendUpdate != null) {
            update = { ...update, ...options.extendUpdate(newValue, txOptions) };
          }

          this.update(update, txOptions);
        } else if (this.kind == "partial") {
          this.update({ [prop.id.toString()]: newValue });
        } else {
          throw new Error(`unsupported kind: ${this.kind}`);
        }
      },
    };
    return row;
  }

  /** Type row for node or partial node */
  rowType(options?: { title?: string; extendWrite?: (newValue: any) => Record<number, any> }): ViewRow {
    const row: ViewRow = {
      type: "view",
      title: options?.title ?? "Type",
      viewType: ViewType.TYPE,
      isFullWidth: false,
      viewProps: { valueType: makeType({ benchType: BenchType.TYPE, isRequired: true }), isInput: true },
      read: () => this.node,
      write: (newType) => {
        let update = getFieldTypeUpdate(this.graph, this.node as FieldData, newType);
        if (options?.extendWrite != null) {
          update = { ...update, ...options.extendWrite(newType) };
        }
        this.update(update, getTransactionOptionsForType(newType));
      },
    };
    return row;
  }

  /** Add field action */
  actionAddField(
    fieldType: FieldType,
    icon: IconData | string = "fas fa-plus",
    options?: { toolPtr?: NodeReferenceData },
  ): ObjectAction {
    return {
      title: `Add ${toCamelName(FieldType, fieldType)}`,
      icon: makeIcon(icon),
      action: (e) => {
        const tool = options?.toolPtr != null ? this.graph.get(options.toolPtr) : null;
        onAddFieldAction(e, fieldType, (tool ?? this.node) as BlockData, this.graph, this.txFactory, {
          dontFocus: true,
        });
      },
    };
  }

  /** Nested object row */
  // rowObject(
  //   propertyId: number,
  //   valueType: TypeIdentity,
  //   options?: { title?: string | false; subtitle?: string; isComputable?: boolean },
  // ): ObjectRow {
  //   const { prop, propName } = this.getProperty(propertyId);

  //   // computed :RunComputedValue
  //   let computedPath: PathData | undefined = undefined;
  //   let computedPathKey: string | undefined = undefined;
  //   if (options?.isComputable) {
  //     computedPath = makePath(
  //       PathElementType.RUN,
  //       propertyReference(NodeType.RUN, RunProperty[propName as any as keyof typeof RunProperty]),
  //     );
  //     computedPathKey = getPathKey(computedPath);
  //   }

  //   // view
  //   const title = options?.title ?? getPropertyTitle(prop);
  //   const row: ObjectRow = {
  //     type: "object",
  //     title: title === false ? undefined : title,
  //     subtitle: options?.subtitle,
  //     isComputable: options?.isComputable ?? false,
  //     prop,
  //     viewType: ViewType.OBJECT,
  //     computedPath,
  //     computedPathKey,
  //     computedType: this.computedType,
  //     viewProps: { valueType: makeType(valueType), isInput: true, isInline: true, isMinimal: true },
  //     isFullWidth: true,
  //     read: () => (this.node as any)[propName],
  //     write: (newValue, options?: ModelValueOptions) => {
  //       if (options == null) {
  //         this.update({ [propName]: newValue });
  //       } else {
  //         const operations: EditOperationData[] = [
  //           {
  //             metatype: ObjectType.EDIT_OPERATION,
  //             type: newValue == null ? EditOperationType.CLEAR : EditOperationType.SET,
  //             path: [propertyId.toString(), ...options.path],
  //             newValuePacked: (newValue as any)?.[options.path[0]],
  //             oldValuePacked: (this.node as any)?.[options.path[0]],
  //           },
  //         ];
  //         this.update(operations, getTransactionOptionsForType(options.field));
  //       }
  //     },
  //   };
  //   return row;
  // }

  /** Line row */
  rowLine(text?: string): LineRow {
    return { type: "line", text };
  }

  /** Icon row */
  rowIcon(icon: IconData | string): IconRow {
    return { type: "icon", icon: makeIcon(icon) };
  }

  /** IO Schema section */
  sectionIO(options?: { title?: string; subtitle?: string; toolPtr?: NodeReferenceData }): ObjectSection {
    return this.section(
      options?.title ?? "Schema",
      [
        { type: "fields-list", fieldType: FieldType.INPUT, toolPtr: options?.toolPtr },
        { type: "icon", icon: makeIcon("fas fa-arrow-down") },
        { type: "fields-list", fieldType: FieldType.OUTPUT, toolPtr: options?.toolPtr },
      ],
      {
        actions: [
          this.actionAddField(FieldType.INPUT, ICON_BY_FIELD_TYPE[FieldType.INPUT], options),
          this.actionAddField(FieldType.OUTPUT, ICON_BY_FIELD_TYPE[FieldType.OUTPUT], options),
        ],
        subtitle: options?.subtitle,
      },
    );
  }

  sectionRun(runOptionsProperty: number) {
    this.section(
      "Run",
      [
        this.rowProperty([runOptionsProperty, RunOptionsProperty.maxAttempts], { title: "Attempts" }),
        this.rowProperty([runOptionsProperty, RunOptionsProperty.suppressFail]),
        this.rowProperty([runOptionsProperty, RunOptionsProperty.modelFamily]),
      ],
      { isDefaultCollapsed: true },
    );
  }

  buildNode() {
    const node = this.node;
    if (node == null) return;
    const nodePtr = toNodeRef(node);
    const graph = this.graph;
    const txFactory = this.txFactory;

    //
    // Blocks
    //

    if (isNode(node, NodeType.BLOCK)) {
      const commonRows: DetailRow[] = [];
      this.section(undefined, commonRows);
      if (node.type != BlockType.TEXT) {
        commonRows.push(this.rowProperty(BlockProperty.text, { title: false, props: { placeholder: "Text..." } }));
      }

      if (node.type == BlockType.CHOICE) {
        this.section("Options", [{ type: "fields-list", fieldType: FieldType.OPTION }], {
          actions: [this.actionAddField(FieldType.OPTION)],
        });
      } else if (node.type == BlockType.DATABASE || node.type == BlockType.MESSAGE) {
        this.section("Members", [{ type: "fields-list", fieldType: FieldType.MEMBER }], {
          actions: [this.actionAddField(FieldType.MEMBER)],
        });
      } else if (RUNNABLE_BLOCK_TYPES.includes(node.type)) {
        this.section("Variables", [{ type: "fields-list", fieldType: FieldType.VARIABLE }], {
          actions: [this.actionAddField(FieldType.VARIABLE)],
        });
        this.sectionIO();
      }
      if (RUNNABLE_BLOCK_TYPES.includes(node.type)) {
        this.sectionRun(BlockProperty.runOptions);
      }
    }

    //
    // Fields
    //
    else if (isNode(node, NodeType.FIELD)) {
      const commonRows: DetailRow[] = [
        this.rowProperty(FieldProperty.text, { title: false, props: { placeholder: "Text..." } }),
      ];
      this.section(undefined, commonRows);
      if (node.type == FieldType.OPTION) {
        // color?
      } else {
        commonRows.push(
          this.rowType({
            extendWrite: (newType) => {
              // also update field name if type changes
              const name = getTypeName(newType);
              return name != null ? { name } : {};
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
                this.txFactory().update(
                  node,
                  { defaultPacked: packValue(newValue, node, { graph: this.graph, wrapScalar: true }) },
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
            this.rowProperty([FieldProperty.constraint, TypeConstraintProperty.minLength], { title: "Minimum Length" }),
          );
          constraintRows.push(
            this.rowProperty([FieldProperty.constraint, TypeConstraintProperty.maxLength], { title: "Maximum Length" }),
          );
        }
        // stringy
        if (node.primitiveType == PrimitiveType.STRING) {
          constraintRows.push(
            this.rowProperty([FieldProperty.constraint, TypeConstraintProperty.startsWith], { title: "Prefix" }),
          );
          constraintRows.push(
            this.rowProperty([FieldProperty.constraint, TypeConstraintProperty.endsWith], { title: "Suffix" }),
          );
          constraintRows.push(
            this.rowProperty([FieldProperty.constraint, TypeConstraintProperty.regex], { title: "Regex" }),
          );
        }
        // number
        if (typeIsNumeric(node)) {
          constraintRows.push(
            this.rowProperty([FieldProperty.constraint, TypeConstraintProperty.minValue], { title: "Minimum" }),
          );
          constraintRows.push(
            this.rowProperty([FieldProperty.constraint, TypeConstraintProperty.maxValue], { title: "Maximum" }),
          );
          constraintRows.push(
            this.rowProperty([FieldProperty.constraint, TypeConstraintProperty.stepValue], { title: "Step" }),
          );
        }
        // node
        if (isNodeType(node.benchType) && SOURCE_NODE_TYPES.includes(node.benchType)) {
          const nodeProperties = PROPERTY_INFOS_BY_TYPE[node.benchType as unknown as NodeType];
          const nodePropertiesEnum = PROPERTY_ENUM_BY_TYPE[node.benchType as unknown as NodeType];
          const subtypeProperty = nodeProperties[(nodePropertiesEnum as any)?.["type"]!];
          if (subtypeProperty?.enumType != null) {
            constraintRows.push(
              this.rowProperty([FieldProperty.constraint, TypeConstraintProperty.nodeSubtypes], {
                title: `${toCamelName(BenchType, node.benchType)} Type`,
                props: {
                  valueType: makeType({
                    kind: TypeKind.ENUM,
                    benchType: subtypeProperty.enumType as unknown as BenchType,
                    isList: true,
                  }),
                },
              }),
            );
          }
          constraintRows.push(
            this.rowProperty([FieldProperty.constraint, TypeConstraintProperty.nodeScopePtr], {
              title: "Scope",
              props: {
                valueType: makeType({ kind: TypeKind.NODE, benchType: BenchType.BLOCK, isList: true }),
              },
            }),
          );
        }

        if (constraintRows.length > 0) {
          this.section("Constraint", constraintRows, { isDefaultCollapsed: true });
        }
      }
    }

    //
    // Actions
    //
    else if (isNode(node, NodeType.ACTION)) {
      const commonRows: DetailRow[] = [];
      this.section(undefined, commonRows);
      commonRows.push(this.rowProperty(ActionProperty.text, { title: false, props: { placeholder: "Text..." } }));

      // common rows
      if (node.type == ActionType.CODE) {
        commonRows.push(this.rowProperty(ActionProperty.code, { isFullWidth: true, isComputable: true }));
      } else if (node.type == ActionType.TOOL) {
        commonRows.push(
          this.rowProperty(ActionProperty.toolPtr, {
            isComputable: true,
            isFullWidth: false,
            extendUpdate: (newValue) => {
              // also update node name if tool changes
              const tool = newValue != null ? this.graph.get(newValue) : null;
              const name = tool != null ? getNodeName(tool) : null;
              return name != null ? { [ActionProperty.name]: name } : {};
            },
          }),
        );
      } else if (node.type == ActionType.FAIL) {
        commonRows.push(this.rowProperty(FailActionProperty.errorTitle, { title: "Title", isComputable: true }));
        commonRows.push(this.rowProperty(FailActionProperty.errorText, { title: "Text", isComputable: true }));
      } else {
        // add all from subproperty enum
        if (this.subpropertyEnum != null) {
          Object.values(this.subpropertyEnum)
            .filter((v) => typeof v == "number")
            .forEach((subproperty) => {
              const { prop } = this.getProperty(subproperty as any);
              if (prop == null || prop.fieldType == FieldType.OUTPUT) return;

              if (prop.valueIsPartial) {
                commonRows.push(
                  this.rowObject(subproperty, makeType({ kind: TypeKind.PARTIAL_OBJECT }), { title: false }),
                );
              } else {
                commonRows.push(this.rowProperty(subproperty, { isComputable: true }));
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
        this.section("Schema", [{ type: "fields-list", fieldType: FieldType.INPUT, toolPtr: node.parentPtr }], {
          subtitle: "(Flow)",
          actions: [
            this.actionAddField(FieldType.INPUT, ICON_BY_FIELD_TYPE[FieldType.INPUT], { toolPtr: node.parentPtr }),
          ],
        });
      } else if (node.type == ActionType.COMPLETE) {
        // ƒlow outputs as action inputs
        this.section(
          "Schema",
          [
            this.rowObject(
              ActionProperty.inputsPacked,
              makeType({
                kind: TypeKind.CUSTOM_OBJECT,
                baseFieldTypes: [FieldType.OUTPUT],
                baseTypePtr: node.parentPtr,
              }),
              { title: false, isComputable: true },
            ),
          ],
          {
            subtitle: "(Flow)",
            actions: [
              this.actionAddField(FieldType.OUTPUT, ICON_BY_FIELD_TYPE[FieldType.OUTPUT], {
                toolPtr: node.parentPtr,
              }),
            ],
          },
        );
      } else if (node.type == ActionType.TOOL && toolPtr != null) {
        // own schema with tool schema
        this.section(
          "Schema",
          [
            // tool variables & inputs
            this.rowObject(
              ActionProperty.variablesPacked,
              makeType({
                kind: TypeKind.CUSTOM_OBJECT,
                baseFieldTypes: [FieldType.VARIABLE],
                baseTypePtr: toolPtr,
              }),
              { title: false, isComputable: true },
            ),
            this.rowObject(
              ActionProperty.inputsPacked,
              makeType({
                kind: TypeKind.CUSTOM_OBJECT,
                baseFieldTypes: [FieldType.INPUT],
                baseTypePtr: toolPtr,
              }),
              { title: false, isComputable: true },
            ),
            // arrow
            this.rowIcon("fas fa-arrow-down"),
            // outputs
            { type: "fields-list", fieldType: FieldType.OUTPUT, toolPtr },
            this.rowLine("Self"),
            { type: "fields-list", fieldType: FieldType.OUTPUT },
          ],
          {
            subtitle: "(Tool)",
            actions: [
              this.actionAddField(FieldType.INPUT, ICON_BY_FIELD_TYPE[FieldType.INPUT]),
              this.actionAddField(FieldType.OUTPUT, ICON_BY_FIELD_TYPE[FieldType.OUTPUT]),
            ],
          },
        );
      } else if (node.type != ActionType.FAIL) {
        // own schema
        this.section(
          "Schema",
          [
            this.rowObject(
              ActionProperty.inputsPacked,
              makeType({
                kind: TypeKind.CUSTOM_OBJECT,
                baseFieldTypes: [FieldType.INPUT],
                baseTypePtr: toolPtr ?? nodePtr ?? undefined,
              }),
              { title: false, isComputable: true },
            ),
            this.rowIcon("fas fa-arrow-down"),
            { type: "fields-list", fieldType: FieldType.OUTPUT },
          ],
          {
            actions: [
              this.actionAddField(FieldType.INPUT, ICON_BY_FIELD_TYPE[FieldType.INPUT]),
              this.actionAddField(FieldType.OUTPUT, ICON_BY_FIELD_TYPE[FieldType.OUTPUT]),
            ],
          },
        );
      }

      // run options
      if (!BOUNDARY_ACTION_TYPES.includes(node.type)) {
        this.sectionRun(ActionProperty.runOptions);
      }
    }

    //
    // Pipes
    //
    else if (isNode(node, NodeType.PIPE)) {
      this.section(undefined, [
        this.rowProperty(PipeProperty.text, { title: false, props: { placeholder: "Text..." } }),
        this.rowProperty(PipeProperty.type),
        this.rowProperty(PipeProperty.color),
        this.rowProperty(PipeProperty.delay),
      ]);
    }

    //
    // Records
    //
    else if (isNode(node, NodeType.RECORD)) {
      this.section(undefined, [
        this.rowProperty(RecordProperty.text, { title: false, props: { placeholder: "Text..." } }),
        this.rowObject(
          RecordProperty.valuePacked,
          makeType({ kind: TypeKind.CUSTOM_OBJECT, baseFieldTypes: [FieldType.MEMBER], baseTypePtr: node.blockPtr }),
          { title: false },
        ),
      ]);
    }
  }

  /** Builds the layout for this object. */
  build() {
    this.buildNode();
  }
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
        valueType: makeType({ benchType: BenchType.TYPE }),
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

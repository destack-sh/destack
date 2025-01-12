import {
  BOUNDARY_ACTION_TYPES,
  getBaseFromNode,
  getPropertyTitle,
  isNodeType,
  isSourceNode,
  RUNNABLE_BLOCK_TYPES,
  SOURCE_NODE_TYPES,
  toCamelName,
} from "@/language/const";
import { useComputedValues } from "@/language/expression";
import {
  createField,
  describeTypeIdentity,
  getFieldTypeUpdate,
  getPropertyType,
  getStorageKey,
  getTypeName,
  makeType,
  makeTypeConstraint,
  TypeIdentity,
  typeIsNumeric,
} from "@/language/field";
import { packSubnode, unpackSubnode } from "@/language/node";
import { getPathKey, makePath } from "@/language/path";
import { ReadNodeGraph } from "@/language/graph";
import {
  getTransactionOptionsForType,
  makeEditFromSubnode,
  Transaction,
  TransactionOptions,
} from "@/language/transaction";
import {
  getPartialObjectNodeType,
  getPartialObjectSubtype,
  packValue,
  unpackPartialNode,
  unpackValue,
} from "@/language/value";
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
  EmptyProperty,
  FailActionProperty,
  FieldData,
  FieldProperty,
  FieldType,
  IconData,
  NodeReferenceData,
  NodeType,
  NodeTypeMapping,
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
import { describeNode, isNode, makeStruct, propertyReference, toNodeRef, toPropertyRef } from "@/proto/wiring";
import { useExistingConnection } from "@/system/connection";
import { canvas, supergraph } from "@/system/globals";
import { getNodeName, ICON_BY_FIELD_TYPE, makeIcon } from "@/ui/icon";
import { pushPopover } from "@/ui/popover";
import { FULL_WIDTH_VIEW_TYPES, getViewForType } from "@/ui/view";
import { assertNever } from "@/utils/functools";
import { computedValue } from "@/utils/ref";
import { ModelValueOptions, ViewProps } from "@/views/common";
import { computed, Ref } from "vue";

export type ObjectAction = {
  title: string;
  icon: IconData;
  action: (e: MouseEvent) => void;
};
export type ObjectSection = {
  key?: string;
  title?: string;
  subtitle?: string;
  rows: Row[];
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
  write: (value: any, options?: any) => void;
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
  field: FieldData;
  options: ModelValueOptions;
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
export type Row = FieldsListRow | ViewRow | PropertyRow | FieldRow | ObjectRow | IconRow | TextRow | LineRow;

export type BaseObjectInfo = {
  isInput: boolean;
  fields: FieldData[];
  delegate: AnyNodeData | null;
  delegateFields: FieldData[];
  graph: ReadNodeGraph;
  update: (
    update: Partial<AnyNodeData> | Record<string, any> | EditOperationData[],
    options?: Partial<ModelValueOptions> & Partial<TransactionOptions>,
  ) => void;
  txFactory: () => Transaction;
};
type NodeInfo<T extends NodeType> = BaseObjectInfo & {
  kind: "node";
  nodeType: T;
  subtype: number | null;
  node: NodeTypeMapping[T];
  subnode: any | null;
};
type PartialNodeInfo = BaseObjectInfo & {
  kind: "partial";
  nodeType: NodeType | null;
  subtype: number | null;
  node: Partial<AnyNodeData> | null;
  subnode: any | null;
  valuePacked: Record<string, any>;
  valueType: TypeData;
  computedType: TypeData | null | undefined;
  computedPath?: PathData;
};
type CustomObjectInfo = BaseObjectInfo & {
  kind: "custom";
  valuePacked: Record<string, any>;
  valueType: TypeData;
  computedType: TypeData | null | undefined;
  computedPath?: PathData;
};
type ObjectInfo = NodeInfo<any> | PartialNodeInfo | CustomObjectInfo;

/** Base class for object layouts */
export abstract class BaseObjectLayout {
  kind: "node" | "partial" | "custom";

  // common
  isInput: boolean;
  fields: FieldData[];
  delegate: AnyNodeData | null;
  delegateFields: FieldData[];
  graph: ReadNodeGraph;
  update: (
    update: Partial<AnyNodeData> | Record<string, any>,
    options?: Partial<ModelValueOptions> & Partial<TransactionOptions>,
  ) => void;
  txFactory: () => Transaction;

  // layout
  sections: ObjectSection[] = [];

  constructor(options: ObjectInfo) {
    this.kind = options.kind;
    this.fields = options.fields;
    this.isInput = options.isInput;
    this.delegate = options.delegate;
    this.delegateFields = options.delegateFields;
    this.graph = options.graph;
    this.update = options.update;
    this.txFactory = options.txFactory;
  }

  /** Add a section to the layout */
  section(
    title: string | undefined,
    rows: Row[],
    options?: {
      key?: string;
      isDefaultCollapsed?: boolean;
      subtitle?: string;
      summary?: string;
      actions?: ObjectAction[];
    },
  ): ObjectSection {
    const section: ObjectSection = {
      key: options?.key ?? (title != null ? `section-${title}-${options?.subtitle ?? ""}` : undefined),
      title,
      subtitle: options?.subtitle,
      rows,
      isDefaultCollapsed: options?.isDefaultCollapsed,
      summary: options?.summary,
      actions: options?.actions,
    };
    this.sections.push(section);
    return section;
  }

  /** Line row */
  rowLine(text?: string): LineRow {
    return { type: "line", text };
  }

  /** Icon row */
  rowIcon(icon: IconData | string): IconRow {
    return { type: "icon", icon: makeIcon(icon) };
  }

  /** Text row */
  rowText(text: string): TextRow {
    return { type: "text", text };
  }

  /** Inline object row */
  rowFieldsInline(
    valuePacked: Record<string, any>,
    fields: FieldData[],
    computedPrefix: PathData | undefined,
    write: (field: FieldData, value: any, options?: ModelValueOptions) => void,
    options?: { title?: string | false; subtitle?: string; isComputable?: boolean; isFullWidth?: boolean },
  ): Row[] {
    const rows: (PropertyRow | FieldRow)[] = [];

    // fields
    for (const field of fields) {
      // content
      const title = options?.title ?? getNodeName(field);
      const fieldKey = getStorageKey(field);
      const view = getViewForType(field, { forcePickerDropdown: true });
      if (view == null) continue;
      const fieldValuePacked = valuePacked[fieldKey];
      const fieldValue = unpackValue(fieldValuePacked, field);

      // computable
      let computedPath: PathData | undefined = undefined;
      let computedPathKey: string | undefined = undefined;
      if (options?.isComputable) {
        computedPath = makePath(...(computedPrefix?.elements ?? []), field);
        computedPathKey = getPathKey(computedPath);
      }

      // row
      const row: FieldRow = {
        type: "field",
        field,
        title: title === false ? undefined : title,
        isComputable: options?.isComputable ?? false,
        computedPath,
        computedPathKey,
        viewType: view.type!,
        viewProps: { ...view, isInput: this.isInput },
        isFullWidth: options?.isFullWidth || FULL_WIDTH_VIEW_TYPES.includes(view.type!),
        options: { field, path: [fieldKey] },
        read: () => fieldValue,
        write: (newValue, options) => write(field, newValue, options),
      };

      rows.push(row);
    }

    return rows;
  }

  abstract make(): void;
}

export abstract class NodeLayout<T extends NodeType> extends BaseObjectLayout {
  kind: "node" | "partial";
  isPartial: boolean;

  // node
  protected nodeType: T | null;
  protected subtype: number | null;
  protected node: Partial<NodeTypeMapping[T]>;
  protected subnode: any | null;
  protected propertyEnum: Record<number, string>;
  protected propertyInfos: Record<number, PropertyInfo>;
  protected subpropertyEnum: Record<number, string> | null;
  protected subpropertyInfos: Record<number, PropertyInfo> | null;
  // value
  protected valuePacked: Record<string, any> | null;
  protected valueType: TypeData | null;
  protected computedType: TypeData | null;
  protected computedPath: PathData | null;

  constructor(options: NodeInfo<T> | PartialNodeInfo) {
    super(options);
    this.kind = options.kind;
    this.isPartial = options.kind == "partial";

    // node
    this.node = options.node as NodeTypeMapping[T];
    this.subnode = options.subnode;
    this.nodeType = options.nodeType as T;
    this.subtype = options.subtype;
    if (this.nodeType != null) {
      this.propertyEnum = PROPERTY_ENUM_BY_TYPE[this.nodeType]!;
      this.propertyInfos = PROPERTY_INFOS_BY_TYPE[this.nodeType as any as ObjectType]!;
      this.subpropertyEnum =
        this.subtype != null ? PROPERTY_ENUM_BY_SUBTYPE[this.nodeType!]?.[this.subtype] : undefined;
      this.subpropertyInfos =
        this.subtype != null ? (PROPERTY_INFOS_BY_SUBTYPE[this.nodeType!]?.[this.subtype] ?? null) : null;
    } else {
      this.propertyEnum = PROPERTY_ENUM_BY_TYPE[ObjectType.EMPTY]!;
      this.propertyInfos = PROPERTY_INFOS_BY_TYPE[ObjectType.EMPTY]!;
      this.subpropertyEnum = null;
      this.subpropertyInfos = null;
    }

    // value
    if (options.kind == "partial") {
      this.valuePacked = options.valuePacked;
      this.valueType = options.valueType;
      this.computedType = options.computedType ?? null;
      this.computedPath = options.computedPath ?? null;
    } else {
      this.valuePacked = null;
      this.valueType = null;
      this.computedType = null;
      this.computedPath = null;
    }
  }

  /** Get the property info for a property */
  getProperty(property: number): { prop: PropertyInfo; propName: string; isSubnode: boolean; path: number[] } {
    let prop: PropertyInfo;
    let propName: string;
    if (this.propertyInfos[property] != null) {
      prop = this.propertyInfos[property];
      propName = this.propertyEnum[property];
      if (prop == null) throw new Error(`no property info for: ${property}`);
      const path = [property];
      return { prop, propName, isSubnode: false, path };
    } else if (this.subpropertyInfos?.[property] != null) {
      prop = this.subpropertyInfos[property];
      propName = this.subpropertyEnum![property];
      if (prop == null) throw new Error(`no property info for: ${property} ${propName}`);
      const path = [EmptyProperty.subnodePacked, this.subtype!, property];
      return { prop, propName, isSubnode: true, path };
    } else {
      throw new Error(`no property info for: ${property}`);
    }
  }

  /** Get property path */
  getPropertyPath(pathIn: number | [number] | [number, number]) {
    if (typeof pathIn == "number") pathIn = [pathIn];
    const {
      prop: rootProp,
      propName: rootPropName,
      path: rootPath,
      isSubnode: isRootSubnode,
    } = this.getProperty(pathIn[0]);
    let prop: PropertyInfo;
    let propNames: [string] | [string, string];
    let path: number[];
    if (pathIn.length == 1) {
      prop = rootProp;
      propNames = [rootPropName];
      path = rootPath;
    } else if (pathIn.length == 2) {
      const rootPropType = getPropertyType(rootProp);
      const rootPropTypeInfos = PROPERTY_INFOS_BY_TYPE[rootPropType?.benchType as unknown as ObjectType];
      const rootPropEnum = PROPERTY_ENUM_BY_TYPE[rootPropType?.benchType as unknown as ObjectType];
      prop = rootPropTypeInfos?.[pathIn[1]];
      const propName = rootPropEnum?.[pathIn[1]];
      if (prop == null || propName == null) throw new Error(`no nested object at: ${pathIn.join(".")}`);
      propNames = [rootPropName, propName];
      path = [...rootPath, pathIn[1]];
    } else {
      assertNever(pathIn);
    }
    return { prop, propNames, path, rootProp, rootPropName, isSubnode: isRootSubnode };
  }

  /** Read a node proeprty */
  readProperty(propNames: [string] | [string, string], isSubnode: boolean) {
    let val;
    if (propNames.length == 1) {
      if (!isSubnode) {
        val = (this.node as any)?.[propNames[0]];
      } else {
        val = (this.subnode as any)?.[propNames[0]];
      }
    } else if (propNames.length == 2) {
      if (!isSubnode) {
        val = (this.node as any)[propNames[0]]?.[propNames[1]];
      } else {
        val = (this.subnode as any)[propNames[0]]?.[propNames[1]];
      }
    } else {
      assertNever(propNames);
    }
    return val;
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
  ): Row {
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
      viewProps: { ...view, ...options?.props, isInput: this.isInput && !options?.isDisabled },
      read: () => {
        // no need to unpack partial node here (because we're always reading from unpacked node)
        const val = this.readProperty(propNames, isSubnode);
        return val ?? options?.default ?? prop.default;
      },
      write: (newValue) => {
        // pack value if partial
        if (this.isPartial) {
          newValue = packValue(newValue, propType, { graph: this.graph });
        }
        const txOptions: TransactionOptions = getTransactionOptionsForType(propType);
        let update: Record<string, any>;
        if (propNames.length == 1) {
          if (!isSubnode) {
            if (!this.isPartial) {
              update = { [rootPropName]: newValue };
            } else {
              update = { [rootProp.id.toString()]: newValue };
            }
          } else {
            update = makeEditFromSubnode(this.node!, {
              metatype: this.nodeType as any,
              type: this.subtype,
              subnode: { [rootPropName]: newValue },
            });
          }
        } else if (propNames.length == 2) {
          if (!isSubnode) {
            const newRootValue = makeStruct({
              metatype: rootProp.referenceStruct,
              ...(this.node as any)[rootPropName],
              [propNames[1]]: newValue,
            });
            if (!this.isPartial) {
              update = { [rootPropName]: newRootValue };
            } else {
              update = { [rootProp.id.toString()]: newRootValue };
            }
          } else {
            throw new Error(`nested subnode property edit not yet implemented`);
          }
        } else {
          assertNever(propNames);
        }
        if (!this.isPartial && options?.extendUpdate != null) {
          update = { ...update, ...options.extendUpdate(newValue, txOptions) };
        }
        this.update(update, txOptions);
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
        let update = getFieldTypeUpdate(this.graph, this.node as unknown as FieldData, newType);
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
  rowObjectNested(
    propertyId: number,
    valueType: TypeIdentity,
    options?: { title?: string | false; subtitle?: string; isComputable?: boolean },
  ): ObjectRow {
    const { prop, path, propNames, isSubnode } = this.getPropertyPath(propertyId);

    // computed
    let computedPath: PathData | undefined = undefined;
    let computedPathKey: string | undefined = undefined;
    if (options?.isComputable) {
      computedPath = makePath(
        PathElementType.RUN, // :RunComputedValue
        propertyReference(NodeType.RUN, RunProperty[propNames[0] as any as keyof typeof RunProperty]),
      );
      computedPathKey = getPathKey(computedPath);
    }

    // content
    const title = options?.title ?? getPropertyTitle(prop);
    const valuePacked = this.readProperty(propNames, isSubnode);

    // view
    const row: ObjectRow = {
      type: "object",
      title: title === false ? undefined : title,
      subtitle: options?.subtitle,
      isComputable: options?.isComputable ?? false,
      prop,
      viewType: ViewType.OBJECT,
      computedPath,
      computedPathKey,
      computedType: this.computedType ?? undefined,
      viewProps: { valueType: makeType(valueType), isInput: true, isInline: true, isMinimal: true },
      isFullWidth: true,
      read: () => valuePacked,
      write: (newValue, options?: ModelValueOptions) => {
        if (this.isPartial) {
          this.update({ [propertyId.toString()]: { ...valuePacked, ...newValue } });
        } else {
          const operations: EditOperationData[] = [
            {
              metatype: ObjectType.EDIT_OPERATION,
              type: newValue == null ? EditOperationType.CLEAR : EditOperationType.SET,
              path: [...path.map((p) => p.toString())],
              newValuePacked: { ...valuePacked, ...newValue },
            },
          ];
          this.update(operations, getTransactionOptionsForType(valueType));
        }
      },
    };
    return row;
  }

  /** Inline object row */
  rowObjectInline(
    propertyId: number,
    valueType: TypeIdentity,
    options?: { title?: string | false; subtitle?: string; isComputable?: boolean; isFullWidth?: boolean },
  ): Row[] {
    const { prop, propName, path } = this.getProperty(propertyId);

    // fields
    let fields: FieldData[];
    if (valueType?.baseTypePtr == null || valueType?.baseTypePtr?.id == this.node?.id) {
      fields = this.fields;
    } else if (valueType?.baseTypePtr?.id == this.delegate?.id) {
      fields = this.delegateFields;
    } else {
      fields = []; // missing delegate, just ignore (probably waiting)
    }
    fields = fields.filter((field) => {
      if (valueType.baseFieldTypes != null && !valueType.baseFieldTypes.includes(field.type)) return false;
      if (field.type == FieldType.OPTION) return false;
      return true;
    });

    // computed
    let computedPrefix: PathData | undefined;
    if (options?.isComputable && this.computedType) {
      computedPrefix = makePath(
        PathElementType.RUN, // :RunComputedValue
        propertyReference(NodeType.RUN, RunProperty[propName as any as keyof typeof RunProperty]),
      );
    }

    // rows
    const valuePacked = this.isPartial ? this.valuePacked : (this.node as any)?.[propName];
    const rows = this.rowFieldsInline(
      valuePacked ?? {},
      fields,
      computedPrefix,
      (field, newValue) => {
        const fieldKey = getStorageKey(field);
        const newFieldValuePacked = packValue(newValue, field, { graph: this.graph });
        const fieldValuePacked = this.isPartial
          ? this.valuePacked?.[fieldKey]
          : (this.node as any)?.[propName]?.[fieldKey];

        if (this.isPartial) {
          this.update(
            { [fieldKey]: newFieldValuePacked },
            { ...getTransactionOptionsForType(field), path: [fieldKey] },
          );
        } else {
          const operations: EditOperationData[] = [
            {
              metatype: ObjectType.EDIT_OPERATION,
              type: newValue == null ? EditOperationType.CLEAR : EditOperationType.SET,
              path: [...path.map((p) => p.toString()), fieldKey],
              newValuePacked: newFieldValuePacked,
              oldValuePacked: fieldValuePacked,
            },
          ];
          this.update(operations, getTransactionOptionsForType(field));
        }
      },
      options,
    );

    // default to fields list
    if (!this.isPartial && rows.length == 0) {
      return [{ type: "fields-list", fieldType: valueType.baseFieldTypes?.[0] ?? FieldType.MEMBER }];
    }

    return rows;
  }

  /** Run options section */
  sectionRunOptions(baseProperty: number) {
    this.section(
      "Run",
      [
        this.rowProperty([baseProperty, RunOptionsProperty.maxAttempts], { title: "Attempts" }),
        this.rowProperty([baseProperty, RunOptionsProperty.suppressFail]),
        this.rowProperty([baseProperty, RunOptionsProperty.modelFamily]),
      ],
      { isDefaultCollapsed: true },
    );
  }
}

export class BlockLayout extends NodeLayout<NodeType.BLOCK> {
  make() {
    const commonRows: Row[] = [];
    this.section(undefined, commonRows);
    if (this.subtype != BlockType.TEXT) {
      commonRows.push(this.rowProperty(BlockProperty.text, { title: false, props: { placeholder: "Text..." } }));
    }

    // schema
    if (!this.isPartial) {
      if (this.subtype == BlockType.CHOICE) {
        this.section("Options", [{ type: "fields-list", fieldType: FieldType.OPTION }], {
          actions: [this.actionAddField(FieldType.OPTION)],
        });
      } else if (this.subtype == BlockType.DATABASE || this.subtype == BlockType.MESSAGE) {
        this.section("Members", [{ type: "fields-list", fieldType: FieldType.MEMBER }], {
          actions: [this.actionAddField(FieldType.MEMBER)],
        });
      } else if (RUNNABLE_BLOCK_TYPES.includes(this.subtype as any)) {
        this.section("Variables", [{ type: "fields-list", fieldType: FieldType.VARIABLE }], {
          actions: [this.actionAddField(FieldType.VARIABLE)],
        });
        this.section(
          "Schema",
          [
            { type: "fields-list", fieldType: FieldType.INPUT },
            { type: "icon", icon: makeIcon("fas fa-arrow-down") },
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
      if (RUNNABLE_BLOCK_TYPES.includes(this.subtype as any)) {
        this.sectionRunOptions(BlockProperty.runOptions);
      }
    }
  }
}

export class FieldLayout extends NodeLayout<NodeType.FIELD> {
  make() {
    const node = this.node!;
    const graph = this.graph;
    const commonRows: Row[] = [
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
      if ((!this.isPartial && this.subtype == FieldType.VARIABLE) || this.subtype == FieldType.MEMBER) {
        const defaultView = getViewForType(node as FieldData, { forcePickerDropdown: true });
        if (defaultView?.type != null) {
          commonRows.push({
            type: "view",
            title: "Default",
            isFullWidth: FULL_WIDTH_VIEW_TYPES.includes(defaultView.type!),
            viewType: defaultView.type,
            viewProps: { ...defaultView, isInput: true },
            read() {
              if (node.defaultPacked == null) return null;
              const defaultUnpacked = unpackValue(node.defaultPacked, node as FieldData, { wrapScalar: true });
              return defaultUnpacked;
            },
            write: (newValue) => {
              this.update(
                { defaultPacked: packValue(newValue, node as FieldData, { wrapScalar: true }) },
                getTransactionOptionsForType(node as FieldData),
              );
            },
          });
        }
      }

      const constraintRows: Row[] = [];
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
      if (this.kind == "node" && typeIsNumeric(node as FieldData)) {
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
}

export class ActionLayout extends NodeLayout<NodeType.ACTION> {
  make() {
    const node = this.node!;

    const commonRows: Row[] = [];
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
              if (this.isPartial) {
                return;
              }
              commonRows.push(
                this.rowObjectNested(subproperty, makeType({ kind: TypeKind.PARTIAL_OBJECT }), {
                  title: "Node",
                  isComputable: true,
                }),
              );
            } else {
              commonRows.push(this.rowProperty(subproperty, { isComputable: true }));
            }
          });
      }
    }

    // schema
    if (!this.isPartial) {
      const nodePtr = toNodeRef(node as ActionData);
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
            ...this.rowObjectInline(
              ActionProperty.inputsPacked,
              makeType({
                kind: TypeKind.CUSTOM_OBJECT,
                baseFieldTypes: [FieldType.OUTPUT],
                baseTypePtr: node.parentPtr,
              }),
              { isComputable: true },
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
            ...this.rowObjectInline(
              ActionProperty.variablesPacked,
              makeType({
                kind: TypeKind.CUSTOM_OBJECT,
                baseFieldTypes: [FieldType.VARIABLE],
                baseTypePtr: toolPtr,
              }),
              { isComputable: true },
            ),
            ...this.rowObjectInline(
              ActionProperty.inputsPacked,
              makeType({
                kind: TypeKind.CUSTOM_OBJECT,
                baseFieldTypes: [FieldType.INPUT],
                baseTypePtr: toolPtr,
              }),
              { isComputable: true },
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
            ...this.rowObjectInline(
              ActionProperty.inputsPacked,
              makeType({
                kind: TypeKind.CUSTOM_OBJECT,
                baseFieldTypes: [FieldType.INPUT],
                baseTypePtr: toolPtr ?? nodePtr ?? undefined,
              }),
              { isComputable: true },
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
    }

    // run options
    if (!BOUNDARY_ACTION_TYPES.includes(this.subtype as any)) {
      this.sectionRunOptions(ActionProperty.runOptions);
    }
  }
}

export class PipeLayout extends NodeLayout<NodeType.PIPE> {
  make() {
    this.section(undefined, [
      this.rowProperty(PipeProperty.text, { title: false, props: { placeholder: "Text..." } }),
      this.rowProperty(PipeProperty.type),
      this.rowProperty(PipeProperty.color),
      this.rowProperty(PipeProperty.delay),
    ]);
  }
}

export class RecordLayout extends NodeLayout<NodeType.RECORD> {
  make() {
    const commonRows: Row[] = [];
    if (this.isPartial) {
      // select block
      commonRows.push(this.rowProperty(RecordProperty.blockPtr, { title: "Database" }));
      // nocheckin: general SourceNode partial properties?
      commonRows.push(this.rowProperty(RecordProperty.name));
    }
    commonRows.push(this.rowProperty(RecordProperty.text, { title: false, props: { placeholder: "Text..." } }));
    if (this.node.blockPtr != null) {
      // value
      commonRows.push(
        ...this.rowObjectInline(
          RecordProperty.valuePacked,
          makeType({
            kind: TypeKind.CUSTOM_OBJECT,
            baseFieldTypes: [FieldType.MEMBER],
            baseTypePtr: this.node.blockPtr,
          }),
        ),
      );
    }
    this.section(undefined, commonRows);
  }
}

export class CustomLayout extends BaseObjectLayout {
  valuePacked: Record<string, any>;
  valueType: TypeData;
  computedPath: PathData | undefined;

  constructor(info: CustomObjectInfo) {
    super(info);
    this.valuePacked = info.valuePacked;
    this.valueType = info.valueType;
    this.delegateFields = info.delegateFields;
    this.computedPath = info.computedPath;
  }

  make() {
    const fields = this.delegateFields.filter((field) => {
      if (this.valueType.baseFieldTypes != null && !this.valueType.baseFieldTypes.includes(field.type)) return false;
      if (field.type == FieldType.OPTION) return false;
      return true;
    });
    this.section(undefined, [
      ...this.rowFieldsInline(this.valuePacked, fields, this.computedPath, (field, newValue, options) => {
        const fieldKey = getStorageKey(field);
        const newFieldValuePacked = packValue(newValue, field, { graph: this.graph });
        this.update({ [fieldKey]: newFieldValuePacked }, { ...getTransactionOptionsForType(field), ...options });
      }),
    ]);
  }
}

export class PartialStubLayout extends BaseObjectLayout {
  nodeType: NodeType | null;
  subtype: number | null;

  constructor(info: PartialNodeInfo) {
    super(info);
    this.nodeType = info.nodeType;
    this.subtype = info.subtype;
  }

  make() {
    this.section(undefined, [
      {
        type: "view",
        isFullWidth: false,
        read: () => undefined,
        write: (value: any) => {
          this.update({ "1": value }, { path: ["1"] });
        },
        title: "Node Type",
        viewType: ViewType.PICKER,
        viewProps: {
          valueType: makeType({ kind: TypeKind.ENUM, benchType: BenchType.NODE_TYPE, isRequired: true }),
          isInput: true,
        },
      },
    ]);
  }
}

export class EmptyLayout extends BaseObjectLayout {
  make() {
    // deliberately empty
  }
}

const NODE_LAYOUT_BY_TYPE = {
  [NodeType.BLOCK]: BlockLayout,
  [NodeType.ACTION]: ActionLayout,
  [NodeType.PIPE]: PipeLayout,
  [NodeType.RECORD]: RecordLayout,
  [NodeType.FIELD]: FieldLayout,
};

/** Use the object layout for a node, partial or custom object */
export function useObjectLayout(options: {
  isInput: Ref<boolean>;
  nodePtr: Ref<NodeReferenceData | undefined>;
  valueType: Ref<TypeData | undefined>;
  valuePacked: Ref<any>;
  computedType?: Ref<TypeData | undefined>;
  updateValuePacked: (update: any, options: any) => void;
  /** Optionally extend or modify partial node layouts after they're built */
  extendPartialLayout?: (layout: BaseObjectLayout, info: ObjectInfo) => BaseObjectLayout;
}) {
  const { valueType, valuePacked, updateValuePacked } = options;

  // kind
  const kind = computed(() => {
    if (valueType.value?.kind == TypeKind.PARTIAL_OBJECT) return "partial";
    if (valueType.value?.kind == TypeKind.CUSTOM_OBJECT) return "custom";
    return "node";
  });

  // node
  const nodePtr = computedValue(() => options.nodePtr.value);
  const { node, connection } = supergraph.getLinkRef(nodePtr);
  const { graph } = useExistingConnection(nodePtr);
  const fields = graph.getChildrenRef(node, NodeType.FIELD);

  // tx
  function txFactory() {
    if (connection.value != null) return connection.value.tx;
    return delegateConnection.tx;
  }

  // node info
  const nodeInfo = computed(() => {
    if (kind.value == "node") {
      if (node.value == null) return null;
      const subtype = (node.value as any).type;
      const subnode =
        (node.value.subnodePacked as any)?.[subtype?.toString()!] != null
          ? (unpackSubnode(node.value.metatype as any, subtype as never, node.value.subnodePacked) as any)
          : null;
      return {
        node: node.value,
        subnode,
        nodeType: node.value.metatype as unknown as NodeType,
        subtype,
      };
    } else if (kind.value == "partial") {
      // nocheckin: partial computed values
      if (options.valueType.value == null) return null;
      const valuePacked = options.valuePacked.value ?? {};
      const nodeType = getPartialObjectNodeType(options.valueType.value, valuePacked);
      const subtype = getPartialObjectSubtype(options.valueType.value, valuePacked);

      let partialNode: Partial<AnyNodeData> | null = null;
      let subnode: any | null = null;
      if (nodeType != null) {
        partialNode = unpackPartialNode(valuePacked, nodeType, subtype);
        subnode =
          (partialNode.subnodePacked as any)?.[subtype?.toString()!] != null
            ? (unpackSubnode(nodeType, subtype as never, partialNode.subnodePacked) as any)
            : null;
      }
      return {
        node: partialNode,
        subnode,
        nodeType,
        subtype,
      };
    }
    return null;
  });

  // delegate
  const delegatePtr = computed(() => {
    if (valueType.value?.baseTypePtr != null) {
      return valueType.value.baseTypePtr;
    }
    const node = nodeInfo.value?.node;
    if (node == null) return null;
    if (isNode(node, NodeType.ACTION) && node.type == ActionType.TOOL) {
      return node.toolPtr;
    } else if (isNode(node, NodeType.ACTION) && (node.type == ActionType.START || node.type == ActionType.COMPLETE)) {
      return node.parentPtr;
    } else if (node != null) {
      return getBaseFromNode(node);
    } else {
      return null;
    }
  });
  const { graph: delegateGraph, connection: delegateConnection } = useExistingConnection(delegatePtr);
  const delegate = delegateGraph.getRef(delegatePtr);
  const delegateFields = delegateGraph.getChildrenRef(delegate, NodeType.FIELD);

  // layout
  const layout = computed(() => {
    if (kind.value == "node") {
      if (nodeInfo.value == null) return null;

      const layoutCls = NODE_LAYOUT_BY_TYPE[nodeInfo.value.nodeType as keyof typeof NODE_LAYOUT_BY_TYPE];
      if (!layoutCls) return null;

      const info: NodeInfo<any> = {
        kind: "node",
        isInput: options.isInput.value,
        node: nodeInfo.value.node,
        subnode: nodeInfo.value.subnode,
        fields: fields.value,
        nodeType: nodeInfo.value.nodeType,
        subtype: nodeInfo.value.subtype,
        delegate: delegate.value,
        delegateFields: delegateFields.value,
        graph,
        update: (update: any, options?: Partial<ModelValueOptions> & Partial<TransactionOptions>) => {
          if (node.value) {
            connection.value?.tx.update(node.value, update, options);
          }
        },
        txFactory,
      };
      const layout = new layoutCls(info as any);
      layout.make();
      return layout;
    } else if (kind.value == "partial") {
      if (nodeInfo.value == null) return null;
      const valuePacked = options.valuePacked.value ?? {};

      const partialInfo: PartialNodeInfo = {
        kind: "partial",
        isInput: options.isInput.value,
        nodeType: nodeInfo.value.nodeType,
        subtype: nodeInfo.value.subtype,
        node: nodeInfo.value.node,
        subnode: nodeInfo.value.subnode,
        valueType: options.valueType.value!,
        valuePacked: valuePacked,
        computedType: computedType.value,
        fields: fields.value,
        delegate: delegate.value,
        delegateFields: delegateFields.value,
        graph,
        update: (update: any, options?: Partial<ModelValueOptions> & Partial<TransactionOptions>) => {
          updateValuePacked({ ...valuePacked, ...update }, options);
        },
        txFactory,
      };
      const nodeLayout = NODE_LAYOUT_BY_TYPE[nodeInfo.value.nodeType as keyof typeof NODE_LAYOUT_BY_TYPE];
      if (nodeLayout == null) {
        // no specific layout (yet)
        const layout = new PartialStubLayout(partialInfo as any);
        layout.make();
        return layout;
      } else {
        const layout = new nodeLayout(partialInfo as any);
        // partial stuff
        layout.section(undefined, [layout.rowProperty(EmptyProperty.metatype, { title: "Node Type" })]);
        // specific node layout
        layout.make();
        return layout;
      }
    } else if (kind.value == "custom") {
      if (options.valueType.value == null) return null;
      const valuePacked = options.valuePacked.value ?? {};

      const layout = new CustomLayout({
        kind: "custom",
        isInput: options.isInput.value,
        valueType: options.valueType.value!,
        valuePacked: valuePacked,
        computedType: computedType.value,
        fields: fields.value,
        delegate: delegate.value,
        delegateFields: delegateFields.value,
        graph,
        update: (update, opts) => {
          updateValuePacked({ ...valuePacked, ...update }, opts);
        },
        txFactory,
      });
      layout.make();
      return layout;
    }

    return null;
  });

  // computed
  const computer = useComputedValues({
    computedValues: computed(() => (isSourceNode(node.value) ? node.value.computedValues : [])),
    update: (computedValues) => {
      if (isSourceNode(node.value)) {
        connection.value?.tx.update(node.value, { computedValues });
      }
    },
  });
  const computedType = computed<TypeData | undefined>(() => {
    if (options.computedType?.value != null) return options.computedType.value;
    if (node.value == null) return undefined;
    return makeType({
      benchType: BenchType.COMPUTED_VALUE,
      isRequired: true,
      constraint: makeTypeConstraint({ nodeScopePtr: [node.value.parentPtr!] }),
    });
  });

  return { layout, node, computer, connection, computedType };
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

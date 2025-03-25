import { supergraph } from "@/globals";
import { getBaseFromNode, getPropertyTitle, isNodeType, isSourceNodeType, toCamelName } from "@/language/core/const";
import { ReadNodeGraph } from "@/language/core/graph";
import { unpackSubnode } from "@/language/core/node";
import {
  getPropertyType,
  getStorageKey,
  getTypeName,
  makeType,
  TypeIdentity,
  typeIsNumeric,
} from "@/language/core/type";
import { getPartialObjectType, packValue, unpackPartialNode, unpackValue } from "@/language/core/value";
import {
  getTransactionOptionsForType,
  makeEditFromSubnode,
  Transaction,
  TransactionOptions,
} from "@/language/runtime/transaction";
import { getFieldTypeUpdate } from "@/language/source/field";
import {
  ActionProperty,
  ActionType,
  AnyNodeData,
  BenchType,
  EditOperationData,
  EditOperationType,
  EmptyProperty,
  FieldData,
  FieldProperty,
  FieldType,
  IconData,
  LinkProperty,
  NodeReferenceData,
  NodeType,
  NodeTypeMapping,
  ObjectType,
  PrimitiveType,
  PROPERTY_ENUM_BY_SUBTYPE,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_SUBTYPE,
  PROPERTY_INFOS_BY_TYPE,
  PropertyInfo,
  RecordProperty,
  TaskProperty,
  ThreadProperty,
  TypeConstraintProperty,
  TypeData,
  TypeKind,
  ViewType,
} from "@/proto/wire";
import { isNode, makeStruct } from "@/proto/wiring";
import { useAutoConnection } from "@/system/connection";
import { getNodeTitle, makeIcon } from "@/ui/icon";
import { FULL_WIDTH_VIEW_TYPES, getViewForType } from "@/ui/view";
import { assertNever } from "@/utils/functools";
import { IS_DEVELOPER_MODE } from "@/utils/globals";
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
export type ClaimsListRow = RowBase & {
  type: "claims-list";
};
export type MembershipListRow = RowBase & {
  type: "membership-list";
  delegatePtr?: NodeReferenceData;
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
  prop: PropertyInfo;
};
export type PropertyRow = Omit<ViewRow, "type"> & {
  type: "property";
  isComputable: boolean;
  prop: PropertyInfo;
};
export type FieldRow = Omit<ViewRow, "type"> & {
  type: "field";
  isComputable: boolean;
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
export type Row =
  | FieldsListRow
  | ClaimsListRow
  | MembershipListRow
  | ViewRow
  | PropertyRow
  | FieldRow
  | ObjectRow
  | IconRow
  | TextRow
  | LineRow;

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
};
type CustomObjectInfo = BaseObjectInfo & {
  kind: "custom";
  valuePacked: Record<string, any>;
  valueType: TypeData;
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
    write: (field: FieldData, value: any, options?: ModelValueOptions) => void,
    options?: { title?: string | false; subtitle?: string; isComputable?: boolean; isFullWidth?: boolean },
  ): Row[] {
    const rows: (PropertyRow | FieldRow)[] = [];

    // fields
    for (const field of fields) {
      // content
      const title = options?.title ?? getNodeTitle(field);
      const fieldKey = getStorageKey(field);
      const view = getViewForType(field, { forcePickerDropdown: true });
      if (view == null) continue;
      const fieldValuePacked = valuePacked[fieldKey];
      const fieldValue = unpackValue(fieldValuePacked, field);

      // row
      const row: FieldRow = {
        type: "field",
        field,
        title: title === false ? undefined : title,
        isComputable: options?.isComputable ?? false,
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
  protected propertyFieldTypes: FieldType[];

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
      this.propertyFieldTypes = options.valueType.propertyFieldTypes ?? [];
    } else {
      this.valuePacked = null;
      this.valueType = null;
      this.propertyFieldTypes = [];
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

  /** Read a node property */
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
        val = (this.node as any)?.[propNames[0]]?.[propNames[1]];
      } else {
        val = (this.subnode as any)?.[propNames[0]]?.[propNames[1]];
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
      extendUpdate?: (newValue: any, options: TransactionOptions) => Record<string, any>;
    },
  ): Row {
    const { path, prop, propNames, rootProp, rootPropName, isSubnode } = this.getPropertyPath(pathIn);

    // view
    const title = options?.title ?? getPropertyTitle(prop);
    const propType = options?.props?.valueType ?? getPropertyType(prop);
    const view = getViewForType(propType);
    if (view == null) {
      return { type: "text", title: title === false ? undefined : title, subtitle: options?.subtitle, text: "No View" };
    }
    const row: PropertyRow = {
      type: "property",
      prop,
      title: title === false ? undefined : title,
      subtitle: options?.subtitle,
      isComputable: options?.isComputable ?? false,
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
          newValue = packValue(newValue, propType);
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

  /** Nested object row */
  rowObjectNested(
    propertyId: number,
    valueType: TypeIdentity,
    options?: { title?: string | false; subtitle?: string; isComputable?: boolean },
  ): ObjectRow {
    const { prop, path, propNames, isSubnode } = this.getPropertyPath(propertyId);

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
      return true;
    });

    // rows
    const valuePacked = this.isPartial ? this.valuePacked : (this.node as any)?.[propName];
    const rows = this.rowFieldsInline(
      valuePacked ?? {},
      fields,
      (field, newValue) => {
        const fieldKey = getStorageKey(field);
        const newFieldValuePacked = packValue(newValue, field);
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
}

export class ChoiceLayout extends NodeLayout<NodeType.CHOICE> {
  make() {
    const commonRows: Row[] = [];
    this.section(undefined, commonRows);

    // schema
    if (!this.isPartial) {
      // this.section("Options", [{ type: "fields-list", fieldType: FieldType.OPTION }], {
      //   actions: [this.actionAddField(FieldType.OPTION)],
      // });
    }
  }
}

export class FieldLayout extends NodeLayout<NodeType.FIELD> {
  make() {
    const node = this.node!;
    const graph = this.graph;
    const commonRows: Row[] = [];
    this.section(undefined, commonRows);
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
    if (this.subtype == FieldType.MEMBER) {
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
    if (isNodeType(node.benchType) && isSourceNodeType(node.benchType)) {
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

export class DatabaseLayout extends NodeLayout<NodeType.DATABASE> {
  make() {
    const commonRows: Row[] = [];
    this.section(undefined, commonRows);

    // schema
    if (!this.isPartial) {
      this.section("Members", [{ type: "fields-list", fieldType: FieldType.MEMBER }], {});
    }
  }
}

abstract class RunnableNodeLayout<T extends NodeType.FLOW | NodeType.KIT | NodeType.ACTION> extends NodeLayout<T> {
  /** Edit the Claims list of Resources/tool Nodes */
  claimRows() {
    const rows: Row[] = [];
    rows.push({ type: "claims-list", title: undefined });
    return rows;
  }
}

export class KitLayout extends RunnableNodeLayout<NodeType.KIT> {
  make() {
    const commonRows: Row[] = [];
    this.section(undefined, commonRows);
    this.section("Tools & Resources", this.claimRows());
  }
}

export class FlowLayout extends RunnableNodeLayout<NodeType.FLOW> {
  make() {
    const commonRows: Row[] = [];
    this.section(undefined, commonRows);

    // schema
    if (!this.isPartial) {
      this.section("Tools & Resources", this.claimRows());
      this.section(
        "Schema",
        [
          { type: "fields-list", fieldType: FieldType.INPUT },
          { type: "icon", icon: makeIcon("fas fa-arrow-down") },
          { type: "fields-list", fieldType: FieldType.OUTPUT },
        ],
        {},
      );
    }
  }
}

export class ActionLayout extends RunnableNodeLayout<NodeType.ACTION> {
  make() {
    const node = this.node!;

    const commonRows: Row[] = [];
    this.section(undefined, commonRows);

    if (node.type == ActionType.START) {
      // flow inputs
      this.section("Schema", [{ type: "fields-list", fieldType: FieldType.INPUT, toolPtr: node.parentPtr }], {
        subtitle: "(Flow)",
      });
    } else if (node.type == ActionType.END) {
      // ƒlow outputs as action inputs
      this.section("Schema", [{ type: "fields-list", fieldType: FieldType.OUTPUT, toolPtr: node.parentPtr }], {
        subtitle: "(Flow)",
      });
    } else if (node.type == ActionType.TOOL) {
      const toolRows: Row[] = [];
      if (node.toolPtr != null) {
        toolRows.push(
          this.rowProperty(ActionProperty.toolPtr, {
            isComputable: true,
            isFullWidth: false,
            extendUpdate: (newValue) => {
              // also update node name if tool changes
              const tool = newValue != null ? this.graph.get(newValue) : null;
              const name = tool != null ? getNodeTitle(tool) : null;
              return name != null ? { name } : {};
            },
          }),
        );
      } else {
        toolRows.push(...this.claimRows());
      }
      this.section("Tools & Resources", toolRows);
      // tool schema
      this.section(
        "Schema",
        [
          { type: "fields-list", fieldType: FieldType.INPUT, toolPtr: node.toolPtr },
          this.rowIcon("fas fa-arrow-down"),
          { type: "fields-list", fieldType: FieldType.OUTPUT, toolPtr: node.toolPtr },
        ],
        {
          subtitle: node.toolPtr != null ? "(Tool)" : undefined,
        },
      );
    } else if (node.type == ActionType.CODE) {
      if (IS_DEVELOPER_MODE.value) {
        commonRows.push(this.rowProperty(ActionProperty.code, { isFullWidth: true, isComputable: true }));
      }

      // code schema
      this.section(
        "Schema",
        [
          { type: "fields-list", fieldType: FieldType.INPUT },
          this.rowIcon("fas fa-arrow-down"),
          { type: "fields-list", fieldType: FieldType.OUTPUT },
        ],
        {},
      );
    }
  }
}

export class LinkLayout extends NodeLayout<NodeType.LINK> {
  make() {
    const commonRows = [this.rowProperty(LinkProperty.type)];
    commonRows.push(this.rowProperty(LinkProperty.color), this.rowProperty(LinkProperty.delay));
    this.section(undefined, commonRows);
  }
}

export class RecordLayout extends NodeLayout<NodeType.RECORD> {
  make() {
    const commonRows: Row[] = [];
    if (this.isPartial) {
      // select block
      commonRows.push(this.rowProperty(RecordProperty.databasePtr, { title: "Database", isComputable: true }));
      // NOTE :Incomplete: generalize SourceNode partial NodeLayout properties?
      commonRows.push(this.rowProperty(RecordProperty.name, { isComputable: true }));
    }
    if (this.node.databasePtr != null) {
      // value
      commonRows.push(
        ...this.rowObjectInline(
          RecordProperty.valuePacked,
          makeType({
            kind: TypeKind.CUSTOM_OBJECT,
            baseFieldTypes: [FieldType.MEMBER],
            baseTypePtr: this.node.databasePtr,
          }),
          { isComputable: this.isPartial },
        ),
      );
    }
    this.section(undefined, commonRows);
  }
}

export class ChannelLayout extends NodeLayout<NodeType.CHANNEL> {
  make() {
    this.section(undefined, []);
    this.section("Members", [{ type: "membership-list", title: undefined }], {});
    this.section("Tools & Resources", [{ type: "claims-list", title: undefined }], {});
  }
}

export class ThreadLayout extends NodeLayout<NodeType.THREAD> {
  make() {
    this.section(undefined, [
      // this.rowProperty(ThreadProperty.mainPagePtr)
    ]);
    this.section("Members", [{ type: "membership-list", title: undefined }], {});
    this.section("Tools & Resources", [{ type: "claims-list", title: undefined }], {});
  }
}

export class TaskLayout extends NodeLayout<NodeType.TASK> {
  make() {
    this.section(undefined, [
      this.rowProperty(TaskProperty.status),
      this.rowProperty(TaskProperty.ownedByPtr, { title: "Owner" }),
      this.rowProperty(TaskProperty.dueAt, { title: "Due" }),
    ]);
  }
}

export class BenchLayout extends NodeLayout<NodeType.BENCH> {
  make() {
    this.section("Members", [{ type: "membership-list", title: undefined, delegatePtr: this.node.mainPackagePtr }], {});
  }
}

/** CustomObject layout with only Fields */
export class CustomLayout extends BaseObjectLayout {
  valuePacked: Record<string, any>;
  valueType: TypeData;

  constructor(info: CustomObjectInfo) {
    super(info);
    this.valuePacked = info.valuePacked;
    this.valueType = info.valueType;
    this.delegateFields = info.delegateFields;
  }

  make() {
    const fields = this.delegateFields.filter((field) => {
      if (this.valueType.baseFieldTypes != null && !this.valueType.baseFieldTypes.includes(field.type)) return false;
      return true;
    });
    this.section(undefined, [
      ...this.rowFieldsInline(this.valuePacked, fields, (field, newValue, options) => {
        const fieldKey = getStorageKey(field);
        const newFieldValuePacked = packValue(newValue, field);
        this.update({ [fieldKey]: newFieldValuePacked }, { ...getTransactionOptionsForType(field), ...options });
      }),
    ]);
  }
}

/** PartialNode layout where we don't know the node type yet */
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

/** Eternal empty nothingness as a layout */
export class EmptyLayout extends BaseObjectLayout {
  make() {
    // deliberately empty
  }
}

const NODE_LAYOUT_BY_TYPE = {
  [NodeType.CHOICE]: ChoiceLayout,
  [NodeType.DATABASE]: DatabaseLayout,
  [NodeType.FLOW]: FlowLayout,
  [NodeType.ACTION]: ActionLayout,
  [NodeType.KIT]: KitLayout,
  [NodeType.LINK]: LinkLayout,
  [NodeType.RECORD]: RecordLayout,
  [NodeType.FIELD]: FieldLayout,
  [NodeType.CHANNEL]: ChannelLayout,
  [NodeType.THREAD]: ThreadLayout,
  [NodeType.TASK]: TaskLayout,
  [NodeType.BENCH]: BenchLayout,
};

/** Use the object layout for a node, partial or custom object */
export function useObjectLayout(options: {
  isInput: Ref<boolean>;
  nodePtr: Ref<NodeReferenceData | undefined>;
  valueType: Ref<TypeData | undefined>;
  valuePacked: Ref<any>;
  updateValuePacked: (update: any, options: any) => void;
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
  const { graph } = useAutoConnection(nodePtr);
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
      if (options.valueType.value == null) return null;
      const valuePacked = options.valuePacked.value ?? {};
      const { nodeType, subtype } = getPartialObjectType(options.valueType.value, valuePacked);

      if (nodeType != null) {
        const { node, subnode } = unpackPartialNode(valuePacked, nodeType, subtype);
        return { node, subnode, nodeType, subtype };
      } else {
        return { node: null, subnode: null, nodeType: null, subtype: null };
      }
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
    } else if (isNode(node, NodeType.ACTION) && (node.type == ActionType.START || node.type == ActionType.END)) {
      return node.parentPtr;
    } else if (node != null) {
      return getBaseFromNode(node);
    } else {
      return null;
    }
  });
  const { graph: delegateGraph, connection: delegateConnection } = useAutoConnection(delegatePtr);
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
      const propertyFieldTypes = options.valueType.value?.propertyFieldTypes ?? [];

      const partialInfo: PartialNodeInfo = {
        kind: "partial",
        isInput: options.isInput.value,
        nodeType: nodeInfo.value.nodeType,
        subtype: nodeInfo.value.subtype,
        node: nodeInfo.value.node,
        subnode: nodeInfo.value.subnode,
        valueType: options.valueType.value!,
        valuePacked: valuePacked,
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
        // partial prefix
        if (propertyFieldTypes.length == 0) {
          layout.section(undefined, [layout.rowProperty(EmptyProperty.metatype, { title: "Node Type" })]);
        }
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

  return { layout, node, connection };
}

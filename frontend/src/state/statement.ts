import { StatementType, TypeHint, TypeTag, type FieldCreateInput, type FieldUpdateInput } from "@/gql/graphql";
import { useActions } from "@/state/actions";
import type { FileHeader, StatementAction } from "@/state/bench";
import { fileContexts, useFileContext, useNavigationContext } from "@/state/file";
import {
  TypeFlag,
  getStatementSubtype,
  useCurrentModule,
  type Field,
  type Statement,
  type Tagging,
  type Trigger,
  type ResolvedField,
  newNodeIdentity,
  useNavigation,
} from "@/state/module";
import { closeTransaction, openTransaction, useOperations } from "@/state/operations";
import { newFieldKey } from "@/state/operations/statement";
import { TYPEHINT_KEYWORD, TYPETAG_KEYWORD } from "@/state/type";
import { INTEGER_ZERO, generateKeyBetween } from "@/utils/fractional";
import { getFieldNameFromTypeName } from "@/utils/functools";
import { syncProperty } from "@/utils/sync";
import {
  AdjustmentsHorizontalIcon as AdjustmentsHorizontalIconOutline,
  ArrowUpRightIcon,
  CircleStackIcon as CircleStackIconOutline,
  CodeBracketSquareIcon as CodeBracketSquareIconOutline,
  RectangleGroupIcon as RectangleGroupIconOutline,
  CpuChipIcon as CpuChipIconOutline,
  SparklesIcon as SparklesIconOutline,
  TagIcon as TagIconOutline,
  PaperAirplaneIcon as PaperAirplaneIconOutline,
  Bars3BottomLeftIcon,
  ViewColumnsIcon as ViewColumnsIconOutline,
} from "@heroicons/vue/24/outline";
import {
  TagIcon as TagIconSolid,
  SparklesIcon as SparklesIconSolid,
  CircleStackIcon as CircleStackIconSolid,
  CodeBracketSquareIcon as CodeBracketSquareIconSolid,
  CpuChipIcon as CpuChipIconSolid,
  AdjustmentsHorizontalIcon as AdjustmentsHorizontalIconSolid,
  RectangleGroupIcon as RectangleGroupIconSolid,
  PaperAirplaneIcon as PaperAirplaneIconSolid,
  ListBulletIcon,
  ViewColumnsIcon as ViewColumnsIconSolid,
  VariableIcon as VariableIcon,
} from "@heroicons/vue/24/solid";
import type { UseElementBoundingReturn } from "@vueuse/core";
import { computed, inject, watch, type Ref } from "vue";

// not using Symbol here to improve hotreload experience (Symbol is not a constant)
export const STATEMENT_CONTEXT = "__statementContext__" as const;

export type StatementContext = {
  depth: Ref<number>;
  xOffset: Ref<number>;
  readonly: Ref<boolean>;
  active: Ref<boolean>;
  focused: Ref<boolean>;
  editing: Ref<boolean>;
  statement: Ref<Statement>;
  file: Ref<FileHeader>;
  destroyed: Ref<boolean>;
  standalone: Ref<boolean>;
  bounding: UseElementBoundingReturn;
  customActions: Ref<StatementAction[] | undefined>;
};

export function useStatementContext() {
  const context = inject<StatementContext>(STATEMENT_CONTEXT);
  if (context == null) {
    throw new Error("StatementContext is not available.");
  }

  // state

  const module = useCurrentModule();
  const statement = context.statement;
  const symbolSubtype: Ref<string | null> = computed(() => getStatementSubtype(statement.value));
  const nav = useNavigationContext();
  const ops = useOperations();

  // basic actions

  const actions = useActions();

  function navigateUp() {
    actions.apply("statement.moveFocusUp");
  }

  function navigateDown() {
    // TODO @UX: navigate down should auto-create? a new statement if there is none below
    actions.apply("statement.moveFocusDown");
  }

  function escape() {
    nav?.value.panel.stopEditingElement(statement.value);
  }

  function deleteSelf() {
    const above = nav?.value?.getAbove(statement.value);
    if (above) {
      nav?.value.panel.focusElement(above);
    }
    ops.statement.softDelete(null, statement.value.id);
  }

  function deleteSelfLeft() {
    const above = nav?.value?.getAbove(statement.value);
    if (above) {
      nav?.value?.panel.focusElement(above, true);
      nav?.value?.statementsComponents[above.id]?.focus("last");
    }
    ops.statement.softDelete(null, statement.value.id);
  }

  function deleteLeft() {
    const above = nav?.value?.getAbove(statement.value);
    if (above) {
      ops.statement.softDelete(null, above.id);
    }
  }

  function insertBelow() {
    actions.apply("statement.insertBelowCurrent");
  }

  function insertAbove() {
    actions.apply("statement.insertAboveCurrent");
  }

  function paste() {
    nav?.value.paste(undefined, statement.value);
  }

  function setCustomActions(actions: Ref<StatementAction[] | undefined>) {
    if (context == null) return;
    watch(
      actions,
      (actions) => {
        context.customActions.value = actions;
      },
      { immediate: true }
    );
  }

  // self mutations

  async function morphToBlank() {
    await ops.statement.morph(null, statement.value.id, { type: statement.value.type }, { type: StatementType.Blank });
  }

  async function morphToComment(text?: string) {
    const tx = openTransaction();
    const updateText = ops.symbol.updateStatementText(tx, statement.value.id, statement.value.text ?? "", text ?? "");
    const morphType = ops.statement.morph(
      tx,
      statement.value.id,
      { type: statement.value.type },
      { type: StatementType.Text }
    );
    closeTransaction(tx);
    await Promise.all([morphType, updateText]);
  }

  async function morphTo(config: {
    type: StatementType | null;
    name?: string;
    rootTypeFlags?: number;
    rootTypeTag?: TypeTag;
  }) {
    if (statement.value.type != StatementType.Blank || config.type == null) {
      throw new Error("cannot morph from non-blank without symbol type: " + statement.value.id);
    }
    const defaults = getDefaultSymbolDefinition(config.type);
    let newTypeTag;
    if (config.rootTypeTag) {
      newTypeTag = config.rootTypeTag;
    } else if (rootTypeTag.value == null) {
      newTypeTag = defaults.rootTypeTag;
    } else {
      newTypeTag = isTypeTagCompatible(rootTypeTag.value, config.type) ? rootTypeTag.value : defaults.rootTypeTag;
    }

    const tx = openTransaction({ name: "morph init", blockPartialUndo: true, collapseUndoToFirst: true });
    await ops.statement.morph(
      tx,
      statement.value.id,
      {
        type: statement.value.type,
        name: undefined,
        lang: statement.value.lang ?? undefined,
        rootTypeTag: statement.value.rootTypeTag ?? undefined,
        rootTypeFlags: statement.value.rootTypeFlags ?? undefined,
      },
      {
        type: config?.type,
        name: config.name,
        lang: config.rootTypeTag ?? defaults.language,
        rootTypeTag: newTypeTag,
        rootTypeFlags: config.rootTypeFlags,
      }
    );
    closeTransaction(tx);
  }

  async function setStatementType(type: StatementType) {
    await ops.statement.morph(null, statement.value.id, { type: statement.value.type }, { type });
  }

  async function setStatementTypeEnum() {
    // morphs to type symbol with an enum as head field
    await ops.statement.morph(
      null,
      statement.value.id,
      {
        type: statement.value.type,
        name: statement.value.name ?? undefined,
        rootTypeTag: statement.value.rootTypeTag ?? undefined,
      },
      {
        type: StatementType.Type,
        name: statement.value.name ?? undefined,
        rootTypeTag: TypeTag.Enum,
      }
    );
  }

  // :EditableSyncDance
  // There is a bit of a delicate dance when syncing these properties since we want to
  // preserve the users local edits, but also want to sync from server/cache when not editing
  // since that includes multiplayer updates and redo/undo data while not editing.
  // That's why we only save the properties while the user is editing, otherwise we would have

  // We can only save if the statement wasn't deleted, and often the statement component owning the
  // statement reference is destroyed before the deletedAt is set, so we also treat the context destroy as delete.
  const isDeleted = computed(() => statement.value.deletedAt != null || context.destroyed.value);

  function syncName(content: Ref<string>, editing: Ref<boolean | undefined>) {
    return syncProperty({
      value: content,
      editing,
      read: () => (content.value = statement.value?.name ?? ""),
      write: () => ops.statement.rename(null, statement.value.id, statement.value.name ?? "", content.value),
      enabled: computed(() => !isDeleted.value),
    });
  }

  function syncCode(content: Ref<string>, editing: Ref<boolean | undefined>) {
    return syncProperty({
      value: content,
      editing,
      read: () => (content.value = statement.value?.code ?? ""),
      write: () => {
        ops.symbol.updateSymbolCode(null, statement.value.id, statement.value.code ?? "", content.value);
      },
      debounceMs: 1000,
      debounceMaxWait: 3000,
      enabled: computed(() => !isDeleted.value),
    });
  }

  function syncText(content: Ref<string>, editing: Ref<boolean | undefined>) {
    return syncProperty({
      value: content,
      editing,
      read: () => (content.value = statement.value?.text ?? ""),
      write: () => {
        ops.symbol.updateStatementText(null, statement.value.id, statement.value.text ?? "", content.value);
      },
      debounceMs: 1000,
      debounceMaxWait: 3000,
      enabled: computed(() => !isDeleted.value),
    });
  }

  function syncDescription(content: Ref<string>, editing: Ref<boolean | undefined>) {
    return syncProperty({
      value: content,
      editing,
      read: () => (content.value = statement.value?.description ?? ""),
      write: () =>
        ops.symbol.updateSymbolDescription(null, statement.value.id, statement.value.description ?? "", content.value),
      enabled: computed(() => !isDeleted.value),
    });
  }

  // typing

  const rootTypeTag = computed(() => statement.value.rootTypeTag);
  const fields = computed(
    () =>
      statement.value.fields
        ?.map((n) => n as Field)
        .filter((n) => n.deletedAt == null)
        .sort((a, b) => (a.orderKey < b.orderKey ? -1 : 1)) ?? []
  );
  const resolvedFields = computed(
    () =>
      statement.value.resolvedFields
        ?.map((n) => n as ResolvedField)
        .map((n) => (n?.fieldCk == null ? null : module.fieldOf(n.fieldCk)))
        .filter((n) => n != null && n.deletedAt == null)
        .map((n) => n as Field)
        .sort((a, b) => (a.orderKey < b.orderKey ? -1 : 1)) ?? []
  );
  const selfFields = computed(() => fields.value?.filter((n) => !(n.flags & TypeFlag.IsUnionWith)) ?? []);
  const baseTypes = computed(
    () => fields.value?.filter((n) => n.flags & TypeFlag.IsUnionWith).map((n) => n as Field) ?? []
  );
  const inheritedFields = computed(() => {
    return (
      resolvedFields.value?.filter(
        (n) => !selfFields.value.find((f) => f.key == n.key) && !(n.flags & TypeFlag.IsUnionWith)
      ) ?? []
    );
  });
  const allFields = computed(() => [...inheritedFields.value, ...selfFields.value]);
  const fieldsByName = computed(() => {
    const fieldsByName: Record<string, Field> = {};
    for (const field of fields.value) {
      if (field.name != null) {
        fieldsByName[field.name] = field as any;
      }
    }
    return fieldsByName;
  });
  const resolvedFieldsByName = computed(() => {
    const fieldsByName: Record<string, Field> = {};
    for (const field of resolvedFields.value) {
      if (field.name != null) {
        fieldsByName[field.name] = field as any;
      }
    }
    return fieldsByName;
  });

  function createField(field: Field) {
    ops.symbol.createField(null, statement.value.id, {
      ...field,
      statement: { __typename: "Statement", id: statement.value.id } as any,
    });
  }

  function createNewField(
    template: Pick<Field, "tag" | "hint" | "flags" | "referenceCk" | "metadata"> & Partial<Field>
  ) {
    const nextOrderKey = generateKeyBetween(
      fields.value?.[fields.value?.length - 1 ?? 0]?.orderKey ?? INTEGER_ZERO,
      null
    );
    const reference = module.statementOf(template.referenceCk ?? "");
    const nameFromReference = reference?.name != null ? getFieldNameFromTypeName(reference?.name) : undefined;
    const name: string =
      TYPEHINT_KEYWORD[template.hint as TypeHint] ?? TYPETAG_KEYWORD[template.tag] ?? nameFromReference ?? "field";
    const identity = newNodeIdentity(module.id.value, "Field");
    if ((template.id != null) != (template.ck != null)) {
      throw new Error("must provide both id and ck or neither");
    }
    const field = makeField({
      projectVersionId: module.id.value,
      ...template,
      id: template.id ?? identity.id,
      ck: template.ck ?? identity.ck,
      name: name.toLowerCase(),
      tag: template.tag,
      hint: template.hint ?? null,
      orderKey: nextOrderKey,
      flags: template.flags ?? 0,
      referenceCk: template.referenceCk ?? null,
    });
    createField(field);
    return field;
  }

  function createUnionField(referenceCk?: string) {
    const nextOrderKey = generateKeyBetween(
      fields.value?.[fields.value?.length - 1 ?? 0]?.orderKey ?? INTEGER_ZERO,
      null
    );
    const field = makeField({
      projectVersionId: module.id.value,
      name: "",
      tag: TypeTag.TypeReference,
      orderKey: nextOrderKey,
      flags: TypeFlag.IsUnionWith,
      referenceCk: referenceCk ?? null,
    });
    createField(field);
    return field;
  }

  function duplicateField(fieldId: string) {
    // :DuplicateField
    const fieldIdx = selfFields.value?.findIndex((m) => m.id === fieldId);
    if (fieldIdx < 0) return;
    const field = selfFields.value?.[fieldIdx];
    const orderKey = generateKeyBetween(field?.orderKey ?? null, selfFields.value?.[fieldIdx + 1]?.orderKey ?? null);
    // "name" => "name 2", "name 2" => "name 3", etc.
    const newName =
      field.name?.search(/\d+$/) != -1 ? field.name?.replace(/\d+$/, (n) => String(Number(n) + 1)) : field.name + " 2";
    const identity = newNodeIdentity(module.id.value, "Field");
    const newFieldNode = {
      ...field,
      ...identity,
      name: newName,
      key: newFieldKey(identity.ck),
      orderKey,
      referenceCk: field.referenceCk ?? null,
    };
    createField(newFieldNode as Field);
    return newFieldNode;
  }

  function updateField(old: { id?: string }, newField: Field) {
    const oldField = fields.value?.find((n) => n.id == old.id) as Field;
    if (!oldField) {
      throw new Error("cannot update field that doesn't exist");
    }
    newField = {
      ...oldField,
      tag: newField.tag ?? oldField.tag,
      hint: newField.hint ?? null,
      name: newField.name ?? oldField.name,
      description: newField.description ?? oldField.description,
      referenceCk: newField.referenceCk ?? null,
      flags: newField.flags,
    } as Field;
    ops.symbol.updateField(null, makeFieldUpdate(oldField as Field), makeFieldUpdate(newField as Field));
  }

  function moveField(field: Field, orderKey: string) {
    const oldField = fields.value?.find((n) => n.id == field.id);
    if (!oldField) {
      throw new Error("cannot move field that doesn't exist");
    }
    ops.symbol.moveField(null, field.id, oldField.orderKey, orderKey);
  }

  function deleteField(field: { id: string }) {
    const oldField = fields.value?.find((n) => n.id == field.id);
    if (!oldField) {
      throw new Error("cannot delete field that doesn't exist");
    }
    ops.symbol.softDeleteField(null, statement.value.id, makeFieldInput(statement.value.id, oldField as Field));
  }

  // basic inline actions
  return {
    // state
    module,
    statement,
    file: context.file,
    depth: context.depth,
    readonly: context.readonly,
    focused: context.focused,
    editing: context.editing,
    xOffset: context.xOffset,
    typeRootTag: rootTypeTag,
    standalone: context.standalone,
    bounding: context.bounding,
    symbolSubtype,
    // actions
    actions,
    setCustomActions,
    navigateUp,
    navigateDown,
    escape,
    morphToBlank,
    morphToComment,
    morpthToSymbol: morphTo,
    setStatementType,
    setStatementTypeEnum,
    syncText,
    syncName,
    syncCode,
    syncDescription,
    deleteSelf,
    deleteSelfLeft,
    deleteLeft,
    insertAbove,
    insertBelow,
    paste,
    // tagging
    tags: computed(() => statement.value.tags.map((t) => t as Tagging).filter((t) => t.deletedAt == null) ?? []),
    // triggers
    triggers: computed(
      () => statement.value.triggers.map((t) => t as Trigger).filter((t) => t.deletedAt == null) ?? []
    ),
    // typing
    fields,
    fieldsByName,
    resolvedFields,
    resolvedFieldsByName,
    selfFields,
    allFields,
    inheritedFields,
    baseTypes,
    createField,
    createNewField,
    createUnionField,
    duplicateField,
    updateField,
    moveField,
    deleteField,
  };
}

function makeFieldInput(id: string, field: Field): FieldCreateInput {
  return {
    statementId: id,
    id: field.id,
    ck: field.ck,
    tag: field.tag,
    hint: field.hint ?? null,
    key: newFieldKey(field.ck),
    orderKey: field.orderKey,
    referenceCk: field.referenceCk ?? null,
    description: field.description ?? null,
    name: field.name ?? null,
    flags: field.flags,
  };
}

function makeFieldUpdate(field: Field): FieldUpdateInput {
  return {
    id: field.id,
    tag: field.tag,
    hint: field.hint ?? null,
    referenceCk: field.referenceCk ?? null,
    description: field.description ?? null,
    name: field.name ?? null,
    flags: field.flags,
  };
}

export function getDefaultSymbolDefinition(type: StatementType): {
  language?: string;
  rootTypeTag?: TypeTag;
} {
  if (type == StatementType.Code) {
    return {
      language: "python",
    };
  } else if (type == StatementType.Task) {
    return {};
  } else if (type == StatementType.Dataset) {
    return {};
  } else if (type == StatementType.Type) {
    // default to struct
    return {
      rootTypeTag: TypeTag.Struct,
    };
  } else {
    // no special content for other symbol types
    return {};
  }
}

export function isTypeTagCompatible(tag: TypeTag, type: StatementType): boolean {
  if (type == StatementType.Code || type == StatementType.Task) {
    return tag == TypeTag.Function;
  } else if (type == StatementType.Dataset) {
    return tag == TypeTag.Struct;
  } else if (type == StatementType.Type) {
    return tag == TypeTag.Struct || tag == TypeTag.Enum;
  } else {
    return false;
  }
}

export function makeField(data: {
  projectVersionId: string;
  id?: string | null;
  ck?: string | null;
  name?: string | null;
  tag: TypeTag;
  hint?: TypeHint | null;
  key?: string;
  orderKey?: string;
  value?: any;
  referenceCk?: string | null;
  flags?: number;
}): Field {
  if ((data.id != null) != (data.ck != null)) {
    throw new Error("must provide both id and ck or neither");
  }
  const identity = newNodeIdentity(data.projectVersionId, "Field");
  const fieldData = {
    id: data.id ?? identity.id,
    ck: data.ck ?? identity.ck,
    name: data.name ?? null,
    tag: data.tag,
    hint: data.hint ?? null,
    key: data.key ?? newFieldKey(data.ck ?? identity.ck),
    orderKey: data.orderKey ?? INTEGER_ZERO,
    referenceCk: data.referenceCk ?? null,
    flags: data.flags ?? 0,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  } as Field;
  return fieldData;
}

export const ANY_FIELD = makeField({ projectVersionId: "00000000-0000-0000-0000-000000000000", tag: TypeTag.Any });

export const ENUM_COLORS = [
  "#f56565", // red-500
  "#ecc94b", // yellow-500
  "#48bb78", // green-500
  "#4299e1", // blue-500
  "#667eea", // indigo-500
  "#9f7aea", // purple-500
  "#ed64a6", // pink-500
  "#6b7280", // gray-500
];

export function getEnumColor(field: { ck: string }) {
  const idx = field.ck.split("").reduce((acc, c) => acc + c.charCodeAt(0), 0);
  return ENUM_COLORS[idx % ENUM_COLORS.length];
}

export const STATEMENT_ICONS_OUTLINE: Partial<Record<StatementType, any>> = {
  [StatementType.Text]: Bars3BottomLeftIcon,
  [StatementType.Tag]: TagIconOutline,
  [StatementType.Task]: SparklesIconOutline,
  [StatementType.Value]: VariableIcon,
  [StatementType.Dataset]: CircleStackIconOutline,
  [StatementType.Code]: CodeBracketSquareIconOutline,
  [StatementType.Flow]: PaperAirplaneIconOutline,
  [StatementType.Model]: CpuChipIconOutline,
  [StatementType.Expectation]: AdjustmentsHorizontalIconOutline,
  [StatementType.Group]: ListBulletIcon,
  [StatementType.Reference]: ArrowUpRightIcon,
};
export const STATEMENT_ICONS_SOLID: Partial<Record<StatementType, any>> = {
  [StatementType.Text]: Bars3BottomLeftIcon,
  [StatementType.Tag]: TagIconSolid,
  [StatementType.Task]: SparklesIconSolid,
  [StatementType.Value]: VariableIcon,
  [StatementType.Dataset]: CircleStackIconSolid,
  [StatementType.Code]: CodeBracketSquareIconSolid,
  [StatementType.Flow]: PaperAirplaneIconSolid,
  [StatementType.Model]: CpuChipIconSolid,
  [StatementType.Expectation]: AdjustmentsHorizontalIconSolid,
  [StatementType.Group]: ListBulletIcon,
  [StatementType.Reference]: ArrowUpRightIcon,
};

export function getStatementIconOutline(type: StatementType, rootTypeTag?: TypeTag | null) {
  if (type == StatementType.Type && rootTypeTag == TypeTag.Struct) {
    return RectangleGroupIconOutline;
  } else if (type == StatementType.Type && rootTypeTag == TypeTag.Enum) {
    return ViewColumnsIconOutline;
  } else {
    return STATEMENT_ICONS_OUTLINE[type];
  }
}

export function getStatementIconSolid(type: StatementType, rootTypeTag?: TypeTag | null) {
  if (type == StatementType.Type && rootTypeTag == TypeTag.Struct) {
    return RectangleGroupIconSolid;
  } else if (type == StatementType.Type && rootTypeTag == TypeTag.Enum) {
    return ViewColumnsIconSolid;
  } else {
    return STATEMENT_ICONS_SOLID[type];
  }
}

export const STATEMENT_TYPE_LABELS: Record<StatementType, string> = {
  [StatementType.Blank]: "Blank",
  [StatementType.Tag]: "Tag",
  [StatementType.Text]: "Text",
  [StatementType.Type]: "Type",
  [StatementType.Task]: "Task",
  [StatementType.Code]: "Code",
  [StatementType.Value]: "Variable",
  [StatementType.Dataset]: "Dataset",
  [StatementType.Model]: "Model",
  [StatementType.Expectation]: "Expectaction",
  [StatementType.Group]: "Group",
  [StatementType.Flow]: "Flow",
  [StatementType.Reference]: "Reference",
};

export function getStatementLabel(type: StatementType, rootTypeTag?: TypeTag | null) {
  if (type == StatementType.Type && rootTypeTag == TypeTag.Struct) {
    return "Class";
  } else if (type == StatementType.Type && rootTypeTag == TypeTag.Enum) {
    return "Choice";
  } else {
    return STATEMENT_TYPE_LABELS[type];
  }
}

export const STATEMENT_TYPE_DESCRIPTIONS: Record<StatementType, string> = {
  [StatementType.Text]: "A plain comment or instruction",
  [StatementType.Type]: "A class, choice or union type",
  [StatementType.Dataset]: "Context, examples, feedback - any records",
  [StatementType.Code]: "Connect, test & customize with Python",
  [StatementType.Task]: "Structured prompt with I/O fields",
  [StatementType.Expectation]: "Tune desired AI behaviour",
  [StatementType.Value]: "Common values for configuration or secrets",
  [StatementType.Reference]: "Reuse another statement",
  [StatementType.Group]: "Relate neighbouring statements",
  [StatementType.Flow]: "Connect code and tasks with triggers",
  [StatementType.Blank]: "Empty statement",
  [StatementType.Model]: "An AI model of any kind",
  [StatementType.Tag]: "Organize and transform statements",
};

export function getStatementDescription(type: StatementType, rootTypeTag?: TypeTag | null) {
  if (type == StatementType.Type && rootTypeTag == TypeTag.Struct) {
    return "A type of an object with some fields";
  } else if (type == StatementType.Type && rootTypeTag == TypeTag.Enum) {
    return "A choice type offering multiple options";
  } else {
    return STATEMENT_TYPE_DESCRIPTIONS[type];
  }
}

export const STATEMENT_STANDALONE_TYPES: StatementType[] = [StatementType.Dataset, StatementType.Code];

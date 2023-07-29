import { useFragment, type FragmentType } from "@/gql";
import {
  StatementType,
  TypeHint,
  TypeTag,
  type Field,
  type FieldCreateInput,
  type FieldUpdateInput,
} from "@/gql/graphql";
import { useActions } from "@/state/actions";
import type { StatementAction } from "@/state/bench";
import { FieldType, FileHeaderType, StatementContentType, TaggingType } from "@/state/fragments";
import { TypeFlag, getSymbolSubtype, useCurrentModule } from "@/state/module";
import { closeTransaction, openTransaction, useOperations } from "@/state/operations";
import { newFieldId, newFieldKey } from "@/state/operations/statement";
import { TYPEHINT_KEYWORD, TYPETAG_KEYWORD } from "@/state/type";
import { INTEGER_ZERO, generateKeyBetween } from "@/utils/fractional";
import { getFieldNameFromTypeName } from "@/utils/functools";
import { syncProperty } from "@/utils/sync";
import {
  AdjustmentsHorizontalIcon,
  ArrowUpRightIcon,
  CircleStackIcon,
  CodeBracketSquareIcon,
  PlayCircleIcon,
  QueueListIcon,
  RectangleGroupIcon,
  ServerStackIcon,
  SparklesIcon,
  TableCellsIcon,
  TagIcon,
} from "@heroicons/vue/24/outline";
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
  statement: Ref<FragmentType<typeof StatementContentType>>;
  file: Ref<FragmentType<typeof FileHeaderType>>;
  destroyed: Ref<boolean>;
  standalone: Ref<boolean>;
  customActions: Ref<StatementAction[] | undefined>;
};

export function useStatementContext() {
  const context = inject<StatementContext>(STATEMENT_CONTEXT);
  if (context == null) {
    throw new Error("StatementContext is not available.");
  }

  // state

  const module = useCurrentModule();
  const statement = computed(() => useFragment(StatementContentType, context.statement.value));
  const file = computed(() => useFragment(FileHeaderType, context.file.value));
  const symbolSubtype: Ref<string | null> = computed(() => getSymbolSubtype(statement.value));

  // basic actions

  const actions = useActions();

  function navigateUp() {
    actions.apply("statement.moveFocusUp");
  }

  function navigateDown() {
    actions.apply("statement.moveFocusDown");
  }

  function escape() {
    actions.apply("statement.stopEditingCurrent");
  }

  function deleteSelf() {
    actions.apply("statement.deleteCurrent");
  }

  function deleteSelfLeft() {
    actions.apply("statement.deleteCurrentLeft");
  }

  function deleteLeft() {
    actions.apply("statement.deleteLeft");
  }

  function insertBelow() {
    actions.apply("statement.insertBelowCurrent");
  }

  function insertAbove() {
    actions.apply("statement.insertAboveCurrent");
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

  const ops = useOperations();

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

  async function morpthToSymbol(config: {
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
        ?.map((n) => useFragment(FieldType, n))
        .filter((n) => n.deletedAt == null)
        .sort((a, b) => (a.orderKey < b.orderKey ? -1 : 1)) ?? []
  );
  const resolvedFields = computed(
    () =>
      statement.value.resolvedFields
        ?.map((n) => useFragment(FieldType, n))
        .filter((n) => n.deletedAt == null)
        .sort((a, b) => (a.orderKey < b.orderKey ? -1 : 1)) ?? []
  );
  const selfFields = computed(() => fields.value?.filter((n) => !(n.flags & TypeFlag.IsUnionWith)) ?? []);
  const baseTypes = computed(
    () => fields.value?.filter((n) => n.flags & TypeFlag.IsUnionWith).map((n) => n as Field) ?? []
  );
  const inheritedFields = computed(() => {
    return resolvedFields.value?.filter((n) => !selfFields.value.find((f) => f.key == n.key)) ?? [];
  });
  const allFields = computed(() => [...inheritedFields.value, ...selfFields.value]);
  const fieldsByName = computed(() => {
    const fieldsByName: Record<string, FragmentType<typeof FieldType>> = {};
    for (const field of fields.value) {
      if (field.name != null) {
        fieldsByName[field.name] = field as any;
      }
    }
    return fieldsByName;
  });
  const resolvedFieldsByName = computed(() => {
    const fieldsByName: Record<string, FragmentType<typeof FieldType>> = {};
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

  function createNewField(template: Pick<Field, "tag" | "hint" | "flags" | "reference" | "metadata">) {
    const nextOrderKey = generateKeyBetween(
      fields.value?.[fields.value?.length - 1 ?? 0]?.orderKey ?? INTEGER_ZERO,
      null
    );
    const nameFromReference =
      template.reference?.name != null ? getFieldNameFromTypeName(template.reference?.name) : undefined;
    const name: string =
      TYPEHINT_KEYWORD[template.hint as TypeHint] ?? TYPETAG_KEYWORD[template.tag] ?? nameFromReference ?? "field";
    const field = makeField({
      name: name.toLowerCase(),
      tag: template.tag,
      hint: template.hint ?? null,
      orderKey: nextOrderKey,
      flags: template.flags ?? 0,
      reference: template.reference as any,
    });
    createField(field);
    return field;
  }

  function createUnionField(referenceId?: string) {
    const nextOrderKey = generateKeyBetween(
      fields.value?.[fields.value?.length - 1 ?? 0]?.orderKey ?? INTEGER_ZERO,
      null
    );
    const field = makeField({
      name: "",
      tag: TypeTag.TypeReference,
      orderKey: nextOrderKey,
      flags: TypeFlag.IsUnionWith,
      reference: referenceId == null ? null : ({ id: referenceId } as any),
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
    const newFieldNode = {
      ...field,
      id: newFieldId(),
      name: newName,
      key: newFieldKey(),
      orderKey,
      reference: field.reference ? { id: field.reference.id } : undefined,
    };
    createField(newFieldNode);
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
      reference: newField.reference,
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
    ops.symbol.softDeleteField(null, statement.value.id, makeFieldInput(statement.value.id, oldField));
  }

  // basic inline actions
  return {
    // state
    module,
    statement,
    file,
    depth: context.depth,
    readonly: context.readonly,
    focused: context.focused,
    editing: context.editing,
    xOffset: context.xOffset,
    typeRootTag: rootTypeTag,
    standalone: context.standalone,
    symbolSubtype,
    // actions
    actions,
    setCustomActions,
    navigateUp,
    navigateDown,
    escape,
    morphToBlank,
    morphToComment,
    morpthToSymbol,
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
    // tagging
    tags: computed(
      () => statement.value.tags.map((t) => useFragment(TaggingType, t)).filter((t) => t.deletedAt == null) ?? []
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
    tag: field.tag,
    hint: field.hint ?? null,
    key: newFieldKey(),
    orderKey: field.orderKey,
    referenceId: field.reference?.id ?? null,
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
    referenceId: field.reference?.id ?? null,
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
  name?: string | null;
  tag: TypeTag;
  hint?: TypeHint | null;
  key?: string;
  orderKey?: string;
  value?: any;
  reference?: { id: string; name?: string } | null;
  flags?: number;
}): Field {
  const fieldData: Field = {
    id: newFieldId(),
    name: data.name ?? null,
    tag: data.tag,
    hint: data.hint ?? null,
    key: data.key ?? newFieldKey(),
    orderKey: data.orderKey ?? INTEGER_ZERO,
    value: data.value ?? null,
    reference: data.reference,
    flags: data.flags ?? 0,
  };
  return fieldData;
}

export type Field = Omit<Field, "revision" | "statement" | "createdAt" | "updatedAt" | "__typename">;

export const NAME_FIELD = makeField({ tag: TypeTag.String, hint: TypeHint.Name });
export const ANY_FIELD = makeField({ tag: TypeTag.Any });

export function getEnumColor(type: { key: string }) {
  /* Generate a strong color for the type */
  // Convert the key to a numerical seed
  const seed = type.key.split("").reduce((acc, char) => {
    return acc * 31 + char.charCodeAt(0);
  }, 0);

  // Generate a random pastel color based on the seed
  const hue = seed % 360;
  const saturation = 70 + (seed % 25); // Range: 70-95
  const lightness = 70;

  return `hsl(${hue}, ${saturation}%, ${lightness}%)`;
}

const icons: Partial<Record<StatementType, any>> = {
  [StatementType.Tag]: TagIcon,
  [StatementType.Task]: SparklesIcon,
  [StatementType.Value]: TableCellsIcon,
  [StatementType.Dataset]: CircleStackIcon,
  [StatementType.Code]: CodeBracketSquareIcon,
  [StatementType.Model]: ServerStackIcon,
  [StatementType.Expectation]: AdjustmentsHorizontalIcon,
  [StatementType.Block]: QueueListIcon,
  [StatementType.Reference]: ArrowUpRightIcon,
};
export function getStatementIcon(type: StatementType, rootTypeTag?: TypeTag | null) {
  if (type == StatementType.Type && rootTypeTag == TypeTag.Struct) {
    return RectangleGroupIcon;
  } else if (type == StatementType.Type && rootTypeTag == TypeTag.Enum) {
    return PlayCircleIcon;
  } else {
    return icons[type];
  }
}

import { useFragment, type FragmentType } from "@/gql";
import {
  ExpectationModifier,
  StatementType,
  TypeHint,
  TypeTag,
  type Field,
  type FieldCreateInput,
  type FieldUpdateInput,
} from "@/gql/graphql";
import { useActions } from "@/state/actions";
import { FieldType, FileHeaderType, StatementContentType, StatementHeaderType } from "@/state/fragments";
import { getSymbolSubtype, TypeFlag, useCurrentModule, type InterpStatement } from "@/state/module";
import { closeTransaction, openTransaction, useOperations } from "@/state/operations";
import { newFieldId, newFieldKey } from "@/state/operations/statement";
import { INTEGER_ZERO } from "@/utils/fractional";
import { syncProperty } from "@/utils/sync";
import { computed, inject, type Ref } from "vue";

// not using Symbol here to improve hotreload experience (Symbol is not a constant)
export const STATEMENT_CONTEXT = "__statementContext__" as const;

export type StatementContext = {
  depth: Ref<number>;
  xOffset: Ref<number>;
  lineNumberBase: Ref<number>;
  readonly: Ref<boolean>;
  active: Ref<boolean>;
  focused: Ref<boolean>;
  editing: Ref<boolean>;
  statement: Ref<FragmentType<typeof StatementContentType>>;
  file: Ref<FragmentType<typeof FileHeaderType>>;
  destroyed: Ref<boolean>;
  standalone: Ref<boolean>;
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

  function tryDeleteLeft() {
    actions.apply("statement.deleteCurrentLeft");
  }

  function insertBelow() {
    actions.apply("statement.insertBelowCurrent");
  }

  function insertAbove() {
    actions.apply("statement.insertAboveCurrent");
  }

  // self mutations

  const ops = useOperations();

  async function morphToBlank() {
    await ops.statement.morph(null, statement.value.id, { type: statement.value.type }, { type: StatementType.Blank });
  }

  async function morphToComment(text?: string) {
    const updateCode = ops.symbol.updateSymbolCode(null, statement.value.id, statement.value.code ?? "", text ?? "");
    const morphType = ops.statement.morph(
      null,
      statement.value.id,
      { type: statement.value.type },
      { type: StatementType.Text }
    );
    await Promise.all([morphType, updateCode]);
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

  async function setModifier(modifier: ExpectationModifier | null) {
    await ops.statement.modify(null, statement.value.id, statement.value.modifier ?? null, modifier);
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

  // :StatementCodeTextReuse
  const syncText = syncCode;

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
  const selfFields = computed(
    () =>
      fields.value?.filter((n) => !(n.flags & TypeFlag.IsUnionWith)).map((n) => module.runtimeTypeOf(n as Field)) ?? []
  );
  const baseTypes = computed(
    () => fields.value?.filter((n) => n.flags & TypeFlag.IsUnionWith).map((n) => n as Field) ?? []
  );
  const inheritedFields = computed(() => {
    return (
      resolvedFields.value
        ?.filter((n) => !selfFields.value.find((f) => f.key == n.key))
        .map((n) => module.runtimeTypeOf(n as Field) as Field) ?? []
    );
  });
  const allFields = computed(() => [...selfFields.value, ...inheritedFields.value]);
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

  async function createField(field: Field) {
    await ops.symbol.createField(null, statement.value.id, { ...field, statementId: statement.value.id });
  }

  async function updateField(oldField: { id?: string }, newField: Field) {
    oldField = fields.value?.find((n) => n.id == oldField.id) as Field;
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
    await ops.symbol.updateField(null, makeFieldUpdate(oldField as Field), makeFieldUpdate(newField as Field));
  }

  async function moveField(field: Field, orderKey: string) {
    const oldField = fields.value?.find((n) => n.id == field.id);
    if (!oldField) {
      throw new Error("cannot move field that doesn't exist");
    }
    await ops.symbol.moveField(null, field.id, oldField.orderKey, orderKey);
  }

  async function deleteField(field: { id: string }) {
    const oldField = fields.value?.find((n) => n.id == field.id);
    if (!oldField) {
      throw new Error("cannot delete field that doesn't exist");
    }
    await ops.symbol.softDeleteField(null, statement.value.id, makeFieldInput(statement.value.id, oldField));
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
    navigateUp,
    navigateDown,
    escape,
    morphToBlank,
    morphToComment,
    morpthToSymbol,
    setModifier,
    setStatementType,
    setStatementTypeEnum,
    syncText,
    syncName,
    syncCode,
    syncDescription,
    deleteSelf,
    tryDeleteLeft,
    insertAbove,
    insertBelow,
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
  reference?: { id: string; name?: string };
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

export const STRING_FIELD = makeField({ tag: TypeTag.String });
export const NAME_FIELD = makeField({ tag: TypeTag.String, hint: TypeHint.Name });
export const ANY_FIELD = makeField({ tag: TypeTag.Any });

export function getEnumColor(type: { key: string }) {
  /* Generate a strong color for the type */
  // Convert the client ID to a numerical seed
  const seed = type.key.split("").reduce((acc, char) => {
    return acc * 31 + char.charCodeAt(0);
  }, 0);

  // Generate a random pastel color based on the seed
  const hue = seed % 360;
  const saturation = 70 + (seed % 25); // Range: 70-95
  const lightness = 70;

  return `hsl(${hue}, ${saturation}%, ${lightness}%)`;
}

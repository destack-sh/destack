import { StatementType, TypeHint, TypeTag, type SearchSort, type SearchQuery } from "@/gql/graphql";
import {
  TypeFlag,
  useCurrentModule,
  type Field,
  type Tagging,
  type Trigger,
  type ResolvedField,
  newNodeIdentity,
  type Statement,
} from "@/state/module";
import { useOperations } from "@/state/operations";
import { newDynamicNodeKey } from "@/state/operations/statement";
import { TYPEHINT_KEYWORD, TYPETAG_KEYWORD } from "@/state/type";
import { INTEGER_ZERO, generateKeyBetween } from "@/utils/fractional";
import { getFieldNameFromTypeName } from "@/utils/functools";
import {
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
  RectangleGroupIcon as RectangleGroupIconSolid,
  PaperAirplaneIcon as PaperAirplaneIconSolid,
  ViewColumnsIcon as ViewColumnsIconSolid,
  VariableIcon as VariableIcon,
} from "@heroicons/vue/24/solid";
import { computed, type Ref } from "vue";

function _computedEmptyIfDisabled<T>(func: () => T, enabled?: Ref<boolean>) {
  return computed(() => (enabled?.value !== false ? func() : []));
}

export function useTags(statement: Ref<Statement>) {
  const module = useCurrentModule();
  const ops = useOperations();

  const tags = computed(() => statement.value.tags?.map((n) => n as Tagging).filter((n) => n.deletedAt == null) ?? []);
  return {
    tags,
  };
}

export function useTriggers(statement: Ref<Statement>) {
  const module = useCurrentModule();
  const ops = useOperations();

  const triggers = computed(
    () => statement.value.triggers?.map((n) => n as Trigger).filter((n) => n.deletedAt == null) ?? []
  );
  return {
    triggers,
  };
}

export function useFieldsState(statement: Ref<Statement>, enabled?: Ref<boolean>) {
  const module = useCurrentModule();

  const fields = _computedEmptyIfDisabled(
    () =>
      statement.value.fields
        ?.map((n) => n as Field)
        .filter((n) => n.deletedAt == null)
        .sort((a, b) => (a.orderKey < b.orderKey ? -1 : 1)) ?? [],
    enabled
  );
  const resolvedFields = _computedEmptyIfDisabled(
    () =>
      statement.value.resolvedFields
        ?.map((n) => n as ResolvedField)
        .map((n) => (n?.fieldCk == null ? null : module.fieldOf(n.fieldCk)))
        .filter((n) => n != null && n.deletedAt == null)
        .map((n) => n as Field)
        .sort((a, b) => (a.orderKey < b.orderKey ? -1 : 1)) ?? [],
    enabled
  );
  const selfFields = _computedEmptyIfDisabled(
    () => fields.value?.filter((n) => !(n.flags & TypeFlag.IsUnionWith)) ?? [],
    enabled
  );
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
  return { fields, resolvedFields, selfFields, baseTypes, inheritedFields, allFields };
}

export function useFields(statement: Ref<Statement>) {
  const module = useCurrentModule();
  const ops = useOperations();

  const { fields, resolvedFields, selfFields, baseTypes, inheritedFields, allFields } = useFieldsState(statement);

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
  const inputs = computed(() => allFields.value.filter((n) => !(n.flags & TypeFlag.IsOutput)));
  const outputs = computed(() => allFields.value.filter((n) => n.flags & TypeFlag.IsOutput));
  const selfInputs = computed(() => selfFields.value.filter((n) => !(n.flags & TypeFlag.IsOutput)));
  const selfOutputs = computed(() => selfFields.value.filter((n) => n.flags & TypeFlag.IsOutput));

  function _createField(field: Field) {
    ops.symbol.createField(null, statement.value.id, {
      ...field,
      statement: { __typename: "Statement", id: statement.value.id } as any,
    });
  }

  function createNewField(template: Pick<Field, "tag" | "hint" | "flags" | "referenceCk" | "value"> & Partial<Field>) {
    const nextOrderKey = generateKeyBetween(
      fields.value?.[fields.value?.length - 1 ?? 0]?.orderKey ?? INTEGER_ZERO,
      null
    );
    const reference = module.statementOf(template.referenceCk ?? "");
    const nameFromReference =
      reference?.name != null && template.tag != TypeTag.Literal
        ? getFieldNameFromTypeName(reference?.name)
        : undefined;
    const name: string =
      template.name ??
      TYPEHINT_KEYWORD[template.hint as TypeHint] ??
      TYPETAG_KEYWORD[template.tag] ??
      nameFromReference ??
      "field";
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
    _createField(field);
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
    _createField(field);
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
      key: newDynamicNodeKey(identity.ck),
      orderKey,
      referenceCk: field.referenceCk ?? null,
    };
    _createField(newFieldNode as Field);
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
      text: newField.text ?? oldField.text,
      referenceCk: newField.referenceCk ?? null,
      flags: newField.flags,
    } as Field;
    ops.symbol.updateField(null, oldField, newField);
  }

  function moveField(field: Field, orderKey: string) {
    const oldField = fields.value?.find((n) => n.id == field.id);
    if (!oldField) {
      throw new Error("cannot move field that doesn't exist");
    }
    ops.symbol.moveField(null, field.id, oldField.orderKey, orderKey);
  }

  function moveFieldTo(field: Field, position: "before" | "after", other: Field) {
    const otherIndex = fields.value?.findIndex((n) => n.id == other.id);
    if (position == "before") {
      const orderKey = generateKeyBetween(fields.value[otherIndex - 1]?.orderKey ?? null, other.orderKey);
      moveField(field, orderKey);
    } else {
      const orderKey = generateKeyBetween(other.orderKey, fields.value[otherIndex + 1]?.orderKey ?? null);
      moveField(field, orderKey);
    }
  }

  function deleteField(field: { id: string }) {
    const oldField = fields.value?.find((n) => n.id == field.id);
    if (!oldField) {
      throw new Error("cannot delete field that doesn't exist");
    }
    ops.symbol.softDeleteField(null, statement.value.id, oldField);
  }

  return {
    fields,
    fieldsByName,
    resolvedFields,
    resolvedFieldsByName,
    selfFields,
    allFields,
    inheritedFields,
    inputs,
    outputs,
    selfInputs,
    selfOutputs,
    baseTypes,
    createNewField,
    createUnionField,
    duplicateField,
    updateField,
    moveField,
    moveFieldTo,
    deleteField,
  };
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
    key: data.key ?? newDynamicNodeKey(data.ck ?? identity.ck),
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
  [StatementType.Variable]: VariableIcon,
  [StatementType.Database]: CircleStackIconOutline,
  [StatementType.Code]: CodeBracketSquareIconOutline,
  [StatementType.Flow]: PaperAirplaneIconOutline,
  [StatementType.Model]: CpuChipIconOutline,
  [StatementType.Reference]: ArrowUpRightIcon,
};
export const STATEMENT_ICONS_SOLID: Partial<Record<StatementType, any>> = {
  [StatementType.Text]: Bars3BottomLeftIcon,
  [StatementType.Tag]: TagIconSolid,
  [StatementType.Task]: SparklesIconSolid,
  [StatementType.Variable]: VariableIcon,
  [StatementType.Database]: CircleStackIconSolid,
  [StatementType.Code]: CodeBracketSquareIconSolid,
  [StatementType.Flow]: PaperAirplaneIconSolid,
  [StatementType.Model]: CpuChipIconSolid,
  [StatementType.Reference]: ArrowUpRightIcon,
};

export function getStatementIconOutline(type: StatementType, tag?: TypeTag | null) {
  if (type == StatementType.Type && tag == TypeTag.Struct) {
    return RectangleGroupIconOutline;
  } else if (type == StatementType.Type && tag == TypeTag.Enum) {
    return ViewColumnsIconOutline;
  } else {
    return STATEMENT_ICONS_OUTLINE[type];
  }
}

export function getStatementIconSolid(type: StatementType, tag?: TypeTag | null) {
  if (type == StatementType.Type && tag == TypeTag.Struct) {
    return RectangleGroupIconSolid;
  } else if (type == StatementType.Type && tag == TypeTag.Enum) {
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
  [StatementType.Variable]: "Variable",
  [StatementType.Database]: "Database",
  [StatementType.Model]: "Model",
  [StatementType.Flow]: "Flow",
  [StatementType.Reference]: "Reference",
};

export function getStatementLabel(type: StatementType, tag?: TypeTag | null) {
  if (type == StatementType.Type && tag == TypeTag.Struct) {
    return "Class";
  } else if (type == StatementType.Type && tag == TypeTag.Enum) {
    return "Choice";
  } else {
    return STATEMENT_TYPE_LABELS[type];
  }
}

export const STATEMENT_TYPE_DESCRIPTIONS: Record<StatementType, string> = {
  [StatementType.Text]: "A plain text comment",
  [StatementType.Type]: "A class, choice or union type",
  [StatementType.Database]: "Examples, state, feedback: any records",
  [StatementType.Code]: "Connect, test & customize with Python",
  [StatementType.Task]: "Structured prompt with I/O fields",
  [StatementType.Variable]: "Common values for configuration or secrets",
  [StatementType.Reference]: "Reuse another statement",
  [StatementType.Flow]: "Connect code and tasks with triggers",
  [StatementType.Blank]: "Empty statement",
  [StatementType.Model]: "An AI model of any kind",
  [StatementType.Tag]: "Organize and transform statements",
};

export function getStatementDescription(type: StatementType, tag?: TypeTag | null) {
  if (type == StatementType.Type && tag == TypeTag.Struct) {
    return "A type of an object with some fields";
  } else if (type == StatementType.Type && tag == TypeTag.Enum) {
    return "A choice type offering multiple options";
  } else {
    return STATEMENT_TYPE_DESCRIPTIONS[type];
  }
}

export type DatabaseStatementProperties = {
  inlineQuery?: string;
  wrapColumns: boolean;
  // local 'view' (because we don't have proper module database view yet, this is the only view)
  sorts?: SearchSort[];
  query?: SearchQuery;
};

export type MorphCommandGroup = {
  name: string;
};

export type MorphCommand = {
  group: MorphCommandGroup;
  identity: MorphIdentity;
  label: string;
  iconOutline: any;
  iconSolid: any;
  description: string;
  aliases?: string[];
  action?: () => void;
};

export type MorphIdentity = {
  type: StatementType;
  tag?: TypeTag | null;
  flags?: TypeFlag | null;
  headingLevel?: number | null;
};

export function getMorphIdentity(statement: Statement): MorphIdentity {
  return {
    type: statement.type,
    tag: statement.tag,
    flags: statement.flags,
    headingLevel: statement.headingLevel,
  };
}

export function canMorphTo(statement: Statement, to: MorphIdentity) {
  return true; // no restrictions yet?
}

export function useStatementMorph(
  statement: Ref<Statement>,
  options?: { query?: Ref<string>; onMorph?: (id: MorphIdentity) => void }
) {
  const GROUPS = {
    BASIC: { name: "Basic statements" },
    LAYOUT: { name: "Layout statements" },
    ADVANCED: { name: "Advanced statements" },
  };

  function simpleStatementCommand(
    group: MorphCommandGroup,
    type: StatementType,
    options?: Omit<MorphIdentity, "type"> & {
      icon?: any;
      label?: string;
      description?: string;
      aliases?: string[];
    }
  ): MorphCommand {
    const identity = { type, ...options };
    return {
      group,
      label: options?.label ?? getStatementLabel(type, options?.tag),
      iconOutline: options?.icon ?? getStatementIconOutline(type, options?.tag),
      iconSolid: options?.icon ?? getStatementIconSolid(type, options?.tag),
      description: options?.description ?? getStatementDescription(type, options?.tag),
      aliases: options?.aliases,
      identity,
    };
  }

  const ops = useOperations();
  function doMorph(
    statement: { id: string; ck: string } & MorphIdentity,
    identity: MorphIdentity & { name?: string | null }
  ) {
    let key = null;
    if ([StatementType.Type, StatementType.Tag, StatementType.Database].includes(identity.type)) {
      key = newDynamicNodeKey(statement.ck);
    }
    ops.statement.morph(null, statement.id, statement, { ...identity, key });
  }

  const commands = computed(() => {
    const commands: MorphCommand[] = [
      // basic statements
      {
        group: GROUPS.BASIC,
        label: "Text",
        aliases: ["comment", "markdown", "title", "header"],
        iconOutline: getStatementIconOutline(StatementType.Text),
        iconSolid: getStatementIconSolid(StatementType.Text),
        description: "Just type for a plain comment",
        identity: { type: StatementType.Text, headingLevel: null },
      },
      simpleStatementCommand(GROUPS.BASIC, StatementType.Type, {
        tag: TypeTag.Struct,
        aliases: ["type", "struct"],
      }),
      simpleStatementCommand(GROUPS.BASIC, StatementType.Type, { tag: TypeTag.Enum, aliases: ["type", "enum"] }),
      simpleStatementCommand(GROUPS.BASIC, StatementType.Database, {
        aliases: ["table", "retrieval", "rag", "samples"],
      }),
      simpleStatementCommand(GROUPS.BASIC, StatementType.Code),
      simpleStatementCommand(GROUPS.BASIC, StatementType.Task, { aliases: ["prompt", "AI", "model", "bot"] }),

      // layout statements
      ...[1, 2, 3].map((level) =>
        simpleStatementCommand(GROUPS.LAYOUT, StatementType.Text, {
          headingLevel: level,
          label: `Heading ${level}`,
          description: `Text with heading ${level}`,
          aliases: [`h${level}`],
        })
      ),

      // advanced statements
      simpleStatementCommand(GROUPS.ADVANCED, StatementType.Variable, { aliases: ["const", "config", "secret"] }),
      // singleStatementCommand(GROUPS.ADVANCED, StatementType.Flow), not fully implemented
      simpleStatementCommand(GROUPS.ADVANCED, StatementType.Tag),
      simpleStatementCommand(GROUPS.ADVANCED, StatementType.Reference),
    ];

    return commands;
  });

  const filteredCommands = computed(() => {
    if (options?.query == null) return commands.value;
    return commands.value
      .filter((command) => canMorphTo(statement.value, command.identity))
      .map((command) => {
        const query = options.query?.value.toLowerCase() as string; // can't change
        const titleMatch = command.label.toLowerCase().includes(query);
        const aliasMatch = command.aliases?.some((alias) => alias.toLowerCase().includes(query));
        const descriptionMatch = command.description.toLowerCase().includes(query);

        return {
          ...command,
          score: titleMatch ? 1 : aliasMatch ? 0.5 : descriptionMatch ? 0.25 : 0,
        };
      })
      .filter((c) => c.score > 0)
      .sort((a, b) => b.score - a.score);
  });

  return { commands, filteredCommands, doMorph };
}

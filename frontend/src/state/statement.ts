import ChoiceTypeIconSolid from "@/components/basic/ChoiceTypeIconSolid.vue";
import ChoiceTypeIconOutline from "@/components/basic/ChoiceTypeIconOutline.vue";
import { StatementType, TypeHint, TypeTag, type Sort, type Conditional } from "@/gql/graphql";
import {
  TypeFlag,
  useCurrentModule,
  type Field,
  type Tagging,
  type Trigger,
  type ResolvedField,
  newNodeIdentity,
  type Statement,
  type InterpStatement,
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
  NoSymbolIcon,
  PuzzlePieceIcon as PuzzlePieceIconOutline,
  EyeIcon as EyeIconOutline,
} from "@heroicons/vue/24/outline";
import {
  TagIcon as TagIconSolid,
  SparklesIcon as SparklesIconSolid,
  CircleStackIcon as CircleStackIconSolid,
  CodeBracketSquareIcon as CodeBracketSquareIconSolid,
  CpuChipIcon as CpuChipIconSolid,
  RectangleGroupIcon as RectangleGroupIconSolid,
  PaperAirplaneIcon as PaperAirplaneIconSolid,
  VariableIcon as VariableIcon,
  PuzzlePieceIcon as PuzzlePieceIconSolid,
  EyeIcon as EyeIconSolid,
} from "@heroicons/vue/24/solid";
import { computed, type Ref } from "vue";
import { useNavigationContext, type CopiedStatement } from "@/state/file";
import uFuzzy from "@leeoniya/ufuzzy";

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

export function useFieldsState(statement: Ref<InterpStatement | Statement | undefined>, enabled?: Ref<boolean>) {
  const module = useCurrentModule();

  const fields = _computedEmptyIfDisabled(
    () =>
      statement.value?.fields
        ?.map((n) => n as Field)
        .filter((n) => n.deletedAt == null)
        .sort((a, b) => (a.orderKey < b.orderKey ? -1 : 1)) ?? [],
    enabled
  );
  const resolvedFields = _computedEmptyIfDisabled(
    () =>
      statement.value?.resolvedFields
        ?.map((n) => n as ResolvedField)
        .map((n) => (n?.fieldCk == null ? null : module.fieldOf(n.fieldCk)))
        .filter((n) => n != null && n.deletedAt == null && !(n.flags & TypeFlag.IS_CONFIG))
        .map((n) => n as Field)
        .sort((a, b) => (a.orderKey < b.orderKey ? -1 : 1)) ?? [],
    enabled
  );
  const selfFields = _computedEmptyIfDisabled(
    () => fields.value?.filter((n) => !(n.flags & TypeFlag.IS_UNION_WITH)) ?? [],
    enabled
  );
  const baseTypes = computed(
    () => fields.value?.filter((n) => n.flags & TypeFlag.IS_UNION_WITH).map((n) => n as Field) ?? []
  );
  const inheritedFields = computed(() => {
    return (
      resolvedFields.value?.filter(
        (n) => !selfFields.value.find((f) => f.key == n.key) && !(n.flags & TypeFlag.IS_UNION_WITH)
      ) ?? []
    );
  });
  const allFields = computed(() => [...inheritedFields.value, ...selfFields.value]);
  return { fields, resolvedFields, selfFields, baseTypes, inheritedFields, allFields };
}

export function useFields(statement: Ref<Statement | InterpStatement | undefined>) {
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
  const inputs = computed(() => allFields.value.filter((n) => !(n.flags & TypeFlag.IS_OUTPUT)));
  const outputs = computed(() => allFields.value.filter((n) => n.flags & TypeFlag.IS_OUTPUT));
  const selfInputs = computed(() => selfFields.value.filter((n) => !(n.flags & TypeFlag.IS_OUTPUT)));
  const selfOutputs = computed(() => selfFields.value.filter((n) => n.flags & TypeFlag.IS_OUTPUT));

  function _createField(field: Field) {
    if (statement.value == null) {
      throw new Error("cannot create field on statement that doesn't exist");
    }
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
    let nameFromReference =
      reference?.name != null && template.tag != TypeTag.Literal
        ? getFieldNameFromTypeName(reference?.name)
        : undefined;
    if (template.flags & TypeFlag.IS_ARRAY) {
      // pluralize
      if (nameFromReference?.endsWith("y")) {
        nameFromReference = nameFromReference.slice(0, -1) + "ies";
      } else {
        nameFromReference = nameFromReference + "s";
      }
    }

    const name: string =
      template.name ??
      TYPEHINT_KEYWORD[template.hint as TypeHint] ??
      TYPETAG_KEYWORD[template.tag] ??
      nameFromReference ??
      "field";sc
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
      flags: TypeFlag.IS_UNION_WITH,
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
    if (statement.value == null) {
      throw new Error("cannot delete field on statement that doesn't exist");
    }
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

export const STATEMENT_ICONS_OUTLINE: Record<StatementType, any> = {
  [StatementType.Blank]: NoSymbolIcon,
  [StatementType.Text]: Bars3BottomLeftIcon,
  [StatementType.Tag]: TagIconOutline,
  [StatementType.Task]: SparklesIconOutline,
  [StatementType.Class]: RectangleGroupIconOutline,
  [StatementType.Choice]: ChoiceTypeIconOutline,
  [StatementType.Variable]: VariableIcon,
  [StatementType.Database]: CircleStackIconOutline,
  [StatementType.Code]: CodeBracketSquareIconOutline,
  [StatementType.Flow]: PaperAirplaneIconOutline,
  [StatementType.Model]: CpuChipIconOutline,
  [StatementType.Group]: PuzzlePieceIconOutline,
  [StatementType.View]: EyeIconOutline,
};
export const STATEMENT_ICONS_SOLID: Record<StatementType, any> = {
  [StatementType.Blank]: NoSymbolIcon,
  [StatementType.Text]: Bars3BottomLeftIcon,
  [StatementType.Tag]: TagIconSolid,
  [StatementType.Class]: RectangleGroupIconSolid,
  [StatementType.Choice]: ChoiceTypeIconSolid,
  [StatementType.Task]: SparklesIconSolid,
  [StatementType.Variable]: VariableIcon,
  [StatementType.Database]: CircleStackIconSolid,
  [StatementType.Code]: CodeBracketSquareIconSolid,
  [StatementType.Flow]: PaperAirplaneIconSolid,
  [StatementType.Model]: CpuChipIconSolid,
  [StatementType.Group]: PuzzlePieceIconSolid,
  [StatementType.View]: EyeIconSolid,
};

export function getStatementIconOutline(type: StatementType) {
  return STATEMENT_ICONS_OUTLINE[type];
}

export function getStatementIconSolid(type: StatementType) {
  return STATEMENT_ICONS_SOLID[type];
}

// :StatementDescriptors
export const STATEMENT_TYPE_TAGS: Partial<Record<StatementType, TypeTag>> = {
  [StatementType.Tag]: TypeTag.Struct,
  [StatementType.Class]: TypeTag.Struct,
  [StatementType.Choice]: TypeTag.Enum,
  [StatementType.Database]: TypeTag.Struct,
  [StatementType.Code]: TypeTag.Function,
  [StatementType.Task]: TypeTag.Function,
  [StatementType.Model]: TypeTag.Struct,
  [StatementType.Flow]: TypeTag.Struct,
};

export const STATEMENT_TYPE_LABELS: Record<StatementType, string> = {
  [StatementType.Blank]: "Blank",
  [StatementType.Tag]: "Tag",
  [StatementType.Text]: "Text",
  [StatementType.Class]: "Class",
  [StatementType.Choice]: "Choice",
  [StatementType.Task]: "Task",
  [StatementType.Code]: "Code",
  [StatementType.Variable]: "Variable",
  [StatementType.Database]: "Database",
  [StatementType.Model]: "Model",
  [StatementType.Flow]: "Flow",
  [StatementType.Group]: "Group",
  [StatementType.View]: "View",
};

export function getStatementLabel(type: StatementType) {
  return STATEMENT_TYPE_LABELS[type];
}

export const STATEMENT_TYPE_DESCRIPTIONS: Record<StatementType, string> = {
  [StatementType.Text]: "A simple text comment or instruction",
  [StatementType.Class]: "A class (or 'type') of object",
  [StatementType.Choice]: "A choice of a fixed set of options",
  [StatementType.Database]: "Real-time, multimodal, relational records.",
  [StatementType.Code]: "Python function or procedure.",
  [StatementType.Task]: "AI model function.",
  [StatementType.Variable]: "Common values for configuration or secrets",
  [StatementType.Flow]: "Connect code and tasks with triggers",
  [StatementType.Blank]: "Empty statement",
  [StatementType.Model]: "An AI model of any kind",
  [StatementType.Tag]: "Organize and transform statements",
  [StatementType.Group]: "Group statements as a unit",
  [StatementType.View]: "Search and view nodes (like records)",
};

export function getStatementDescription(type: StatementType) {
  return STATEMENT_TYPE_DESCRIPTIONS[type];
}

export type DatabaseStatementProperties = {
  inlineQuery?: string;
  wrapColumns: boolean;
  // local 'view' (because we don't have proper module database view yet, this is the only view)
  sorts?: Sort[];
  filters?: Conditional[];
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
  template?: InterpStatement;
};

export type MorphIdentity = {
  type: StatementType;
  headingLevel?: number | null;
  versioned?: boolean;
};

export function getMorphIdentity(statement: Statement): MorphIdentity {
  return {
    type: statement.type,
    headingLevel: statement.headingLevel,
  };
}

export function canMorphTo(statement: Statement, to: MorphIdentity) {
  return true; // no restrictions yet?
}

const MORPH_GROUPS = {
  BASIC: { name: "Basic statements" },
  LAYOUT: { name: "Layout statements" },
  ADVANCED: { name: "Advanced statements" },
  TEMPLATES: { name: "Templates" },
};
const MORPH_GROUPS_ORDER = Object.values(MORPH_GROUPS).map((g) => g.name);

export function useStatementMorph(
  statement: Ref<Statement>,
  active: Ref<boolean>,
  options?: {
    query?: Ref<string>;
    onMorph?: (id: MorphIdentity) => void;
    includeTemplates?: boolean;
  }
) {
  function simpleStatementCommand(
    group: MorphCommandGroup,
    type: StatementType,
    options?: Omit<MorphIdentity, "type" | "moduleId"> & {
      icon?: any;
      label?: string;
      description?: string;
      aliases?: string[];
    }
  ): MorphCommand {
    const identity = { type, ...options };
    return {
      group,
      label: options?.label ?? getStatementLabel(type),
      iconOutline: options?.icon ?? getStatementIconOutline(type),
      iconSolid: options?.icon ?? getStatementIconSolid(type),
      description: options?.description ?? getStatementDescription(type),
      aliases: options?.aliases,
      identity,
    };
  }

  const ops = useOperations();
  function doMorph(
    target: { id: string; ck: string } & MorphIdentity,
    identity: MorphIdentity & { name?: string | null },
    command: MorphCommand
  ) {
    if (command.template != null) {
      // copy descendant statements from source module into current module
      const sourceModule = module.modules.value.find((m) => m?.statementsById[command.template?.id] != null);
      if (sourceModule == null) throw new Error("source template module not found");
      const descendants: InterpStatement[] = [command.template];
      const walkDescendants = (statement: InterpStatement) => {
        sourceModule.statementsByParentId[statement.id]?.forEach((s) => {
          descendants.push(s);
          walkDescendants(s);
        });
      };
      walkDescendants(command.template);

      const copied = descendants.map(
        (s) =>
          ({
            id: s.id,
            ck: s.ck,
            orderKey: s.orderKey,
            parentId: s.parent?.id,
            parentInCopy: descendants.find((d) => d.id == s.parent?.id),
          } as CopiedStatement)
      );
      nav?.value.paste(copied, statement.value);
    } else {
      // create blank statement of that type as usual
      let key = null;
      if (
        [StatementType.Class, StatementType.Choice, StatementType.Tag, StatementType.Database].includes(identity.type)
      ) {
        key = newDynamicNodeKey(target.ck);
      }
      ops.statement.morph(null, target.id, target, { ...identity, key });
    }
  }

  const module = useCurrentModule();
  const nav = useNavigationContext();
  const commands = computed(() => {
    if (!active.value) return [];
    const commands: MorphCommand[] = [
      // basic statements
      {
        group: MORPH_GROUPS.BASIC,
        label: "Text",
        aliases: ["comment", "markdown", "title", "header"],
        iconOutline: getStatementIconOutline(StatementType.Text),
        iconSolid: getStatementIconSolid(StatementType.Text),
        description: "Just type for a plain comment",
        identity: { type: StatementType.Text, headingLevel: null },
      },
      simpleStatementCommand(MORPH_GROUPS.BASIC, StatementType.Class, { aliases: ["type", "struct"] }),
      simpleStatementCommand(MORPH_GROUPS.BASIC, StatementType.Choice, { aliases: ["type", "enum"] }),
      simpleStatementCommand(MORPH_GROUPS.BASIC, StatementType.Database, {
        aliases: ["table", "retrieval", "rag", "samples", "context"],
      }),
      simpleStatementCommand(MORPH_GROUPS.BASIC, StatementType.Code),
      simpleStatementCommand(MORPH_GROUPS.BASIC, StatementType.Task, { aliases: ["prompt", "AI", "model", "bot"] }),

      // layout statements
      ...[1, 2, 3].map((level) =>
        simpleStatementCommand(MORPH_GROUPS.LAYOUT, StatementType.Text, {
          headingLevel: level,
          label: `Heading ${level}`,
          description: `Text with heading ${level}`,
          aliases: [`h${level}`],
        })
      ),

      // advanced statements
      simpleStatementCommand(MORPH_GROUPS.ADVANCED, StatementType.Variable, {
        aliases: ["const", "config", "secret", "let"],
      }),
      simpleStatementCommand(MORPH_GROUPS.ADVANCED, StatementType.Tag),
      simpleStatementCommand(MORPH_GROUPS.ADVANCED, StatementType.Group),
    ];

    if (options?.includeTemplates) {
      const templateTag = module.tags.value?.find((t) => t.name == "template");
      const templateStatements = module.allStatements.value.filter((s) =>
        s.tags.find((t) => t.key == templateTag?.key)
      );
      // sort templates in same order of types, then by name
      const statementTypeIndex: Partial<Record<StatementType, number>> = {};
      commands.forEach((c, i) => (statementTypeIndex[c.identity.type] = i));
      templateStatements.sort(
        (a, b) =>
          (statementTypeIndex[a.type] as number) - (statementTypeIndex[b.type] as number) ??
          (a.name ?? "").localeCompare(b.name ?? "")
      );
      templateStatements.forEach((s) => {
        let description = s.text ?? "";
        if (description.endsWith(".")) {
          description = description.slice(0, -1);
        }
        commands.push({
          group: MORPH_GROUPS.TEMPLATES,
          label: s.name as string,
          aliases: [s.type], // so it appears when searching for the statement type
          iconOutline: getStatementIconOutline(s.type),
          iconSolid: getStatementIconSolid(s.type),
          description,
          identity: { type: s.type, headingLevel: s.headingLevel }, // should have .versioned here too but currently not in InterpStatement
          template: s,
        });
      });
    }

    return commands;
  });

  const uf = new uFuzzy({ intraMode: 0 });
  const filteredCommands = computed(() => {
    if (options?.query == null) return commands.value;
    const [idxs, info, order] = uf.search(
      commands.value
        .filter((command) => canMorphTo(statement.value, command.identity))
        .map((c) => {
          let hay = c.label;
          if (c.aliases != null) {
            hay += " " + c.aliases.join(" ");
          }
          if (c.description != null) {
            hay += " " + c.description;
          }
          return hay;
        }),
      options.query.value
    );
    if (idxs && order) {
      // put basic statements first, retain order of results
      return order
        .map((i) => commands.value[idxs[i]])
        .sort((a, b) => MORPH_GROUPS_ORDER.indexOf(a.group.name) - MORPH_GROUPS_ORDER.indexOf(b.group.name));
    }
    return commands.value;
  });

  return { commands, filteredCommands, doMorph };
}

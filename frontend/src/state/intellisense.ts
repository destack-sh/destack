import { useFragment, type FragmentType } from "@/gql";
import { StatementModifier, StatementType, SymbolType } from "@/gql/graphql";
import { FileHeaderType, StatementContentType, StatementHeaderType } from "@/state/fragments";
import { computed, reactive, type Ref } from "vue";

export type StatementMetadata = {
  isRedefinition: boolean;
  isDefinition: boolean;
  isReference: boolean;
  isArgument: boolean;
  isParameter: boolean;
  isImport: boolean;
  isComment: boolean;
  isCommented: boolean;
  isDeleted: boolean;
  isRunnable: boolean;
  isAlias: boolean;
  requirementPath?: string | null;
  importPath?: string | null;
};

export function useStatementMetadata(
  fileRef: Ref<FragmentType<typeof FileHeaderType>>,
  statementRef: Ref<FragmentType<typeof StatementContentType>>
): StatementMetadata {
  const statement = computed(() => useFragment(StatementContentType, statementRef.value));
  const reference = computed(() => useFragment(StatementHeaderType, statement.value?.reference));

  const isRedefinition = computed(() => statement.value?.type == StatementType.Redefinition);
  const isDefinition = computed(() => statement.value?.type == StatementType.Definition || isRedefinition.value);
  const isReference = computed(() => statement.value?.type == StatementType.Reference || isRedefinition.value);
  const isParameter = computed(() => isReference.value && statement.value.modifier == StatementModifier.With);
  const isArgument = computed(() => isDefinition.value && statement.value.modifier == StatementModifier.With);
  const isImport = computed(() => statement.value?.type == StatementType.Import);
  const isComment = computed(() => statement.value?.type == StatementType.Comment);
  const isCommented = computed(() => statement.value?.commented);
  const isRunnable = computed(
    () =>
      !isImport.value &&
      (statement.value?.symbolType == SymbolType.Code || statement.value?.symbolType == SymbolType.Task)
  );
  const isDeleted = computed(() => statement.value?.deletedAt != null);

  const isAlias = computed(
    () => isImport.value && reference.value != null && reference.value?.name != statement.value.name
  );
  const requirementPath = computed(() => "???");
  const importPath = computed(() => "???");

  return reactive({
    isRedefinition,
    isDefinition,
    isReference,
    isArgument,
    isParameter,
    isImport,
    isComment,
    isCommented,
    isDeleted,
    isRunnable,
    isAlias,
    requirementPath,
    importPath,
  });
}

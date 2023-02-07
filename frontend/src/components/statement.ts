import { useFragment, type FragmentType } from "@/gql";
import { StatementModifier, StatementType } from "@/gql/graphql";
import { useActions } from "@/state/actions";
import { FileHeaderType, StatementContentType, StatementHeaderType } from "@/state/fragments";
import { useOperations } from "@/state/operations";
import { useDebounceFn } from "@vueuse/shared";
import { computed, inject, watch, type Ref } from "vue";

export const STATEMENT_CONTEXT = Symbol();

export type StatementContext = {
  depth: number;
  xOffset: number;
  lineNumberBase: number;
  readonly: boolean;
  focused: boolean;
  editing: boolean;
  statement: FragmentType<typeof StatementContentType>;
  file: FragmentType<typeof FileHeaderType>;
  reference: FragmentType<typeof StatementHeaderType> | null;
};

export function useStatementContext() {
  const context = inject<Ref<StatementContext>>(STATEMENT_CONTEXT);
  if (context == null) {
    throw new Error("StatementContext is not available.");
  }
  const statement = computed(() => useFragment(StatementContentType, context.value.statement));
  const file = computed(() => useFragment(FileHeaderType, context.value.file));
  const reference = computed(() => useFragment(StatementHeaderType, context.value.reference));

  const actions = useActions();
  const operations = useOperations();

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

  function insertBelow() {
    actions.apply("statement.insertBelowCurrent");
  }

  // modifications

  async function morphToComment() {
    await operations.statement.morph(
      statement.value.id,
      { type: statement.value.type, symbolType: statement.value.symbolType ?? undefined },
      { type: StatementType.Comment }
    );
  }

  async function morphSetModifier(modifier: StatementModifier | null) {
    await operations.statement.modify(statement.value.id, statement.value.modifier ?? null, modifier);
  }

  // one-way syncs from current state to backend (:Singleplayer)

  function syncCode(content: Ref<string>) {
    function saveCode() {
      operations.symbol.updateStatementCode(statement.value.id, statement.value.code ?? "", content.value);
    }
    watch(content, useDebounceFn(saveCode, 200, { maxWait: 500 }));
  }

  function syncDescription(content: Ref<string>) {
    function saveDescription() {
      operations.symbol.updateStatementDescription(
        statement.value.id,
        statement.value.description ?? "",
        content.value
      );
    }
    watch(content, useDebounceFn(saveDescription, 200, { maxWait: 500 }));
  }

  return {
    // state
    statement,
    file,
    reference,
    depth: computed(() => context.value.depth),
    readonly: computed(() => context.value.readonly),
    focused: computed(() => context.value.focused),
    editing: computed(() => context.value.editing),
    xOffset: computed(() => context.value.xOffset),
    lineNumberBase: computed(() => context.value.lineNumberBase),
    // actions
    actions,
    navigateUp,
    navigateDown,
    escape,
    morphToComment,
    morphSetModifier,
    syncCode,
    syncDescription,
    deleteSelf,
    insertBelow,
  };
}

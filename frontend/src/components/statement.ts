import { useFragment, type FragmentType } from "@/gql";
import { useActions } from "@/state/actions";
import { FileHeaderType, StatementContentType, StatementHeaderType } from "@/state/fragments";
import { computed, inject, type Ref } from "vue";

export const STATEMENT_CONTEXT = Symbol();

export type StatementContext = {
  depth: number;
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

  function navigateUp() {
    actions.apply("statement.moveFocusUp");
  }

  function navigateDown() {
    actions.apply("statement.moveFocusDown");
  }

  function escape() {
    actions.apply("statement.stopEditingCurrent");
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
    // actions
    actions,
    navigateUp,
    navigateDown,
    escape,
  };
}

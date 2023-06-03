<script lang="ts" setup>
import { useNavigationGrid } from "@/composables/useGrid";
import { StatementType } from "@/gql/graphql";
import { useBenchState, type ViewId } from "@/state/bench";
import { useCurrentModule, useNavigation, type InterpStatement } from "@/state/module";
import { SYMBOL_TYPE_KEYWORD } from "@/state/type";
import { computed, nextTick } from "vue";

const props = defineProps<{ showAllStatements?: boolean }>();
const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
}>();

const runtime = useCurrentModule();
const bench = useBenchState();
const nav = useNavigation();

const filteredStatements = computed(() => {
  if (bench.focusedFileId == null) {
    return undefined;
  }
  return Object.values(runtime.moduleIndex.value?.statementsById ?? {}).filter(
    (s) => s.file.id == bench.focusedFileId && s.type == StatementType.Definition
  );
});

const statementsGrid = useNavigationGrid<"name", HTMLElement>(
  computed(() => ["name"]),
  computed(() => filteredStatements.value ?? []),
  {
    gridNavigateUp: () => emit("navigateUp"),
    gridNavigateDown: () => emit("navigateDown"),
  }
);

function focusStatement(statement: InterpStatement) {
  const focusedViewId = bench.focusedViewId;
  nav.focus(statement);
  bench.focusView(focusedViewId as ViewId); // keep focused view
  nextTick(() => nav.focus(statement));
}

function focusStatementAndGoThere(statement: InterpStatement) {
  nav.focus(statement);
}

function focus(target: "first" | "last" = "first") {
  statementsGrid.focus(target == "first" ? 0 : -1, "name");
}

function blur() {
  statementsGrid.blur();
}

defineExpose({
  count: computed(() => filteredStatements.value?.length),
  focus,
  blur,
});
</script>
<template>
  <ul v-if="filteredStatements != null" role="list" class="flex flex-col py-1 text-sm">
    <li
      v-for="statement in filteredStatements"
      :key="statement.id"
      :ref="(ref) => statementsGrid.registerColumnRef(statement.id, 'name', ref)"
      tabindex="-1"
      class="flex flex-row gap-1 border border-transparent px-3 py-0.5 text-gray-700 outline-none hover:cursor-pointer hover:bg-orange-100 focus:border-orange-600"
      :class="{
        'bg-orange-100 text-orange-600': statement.id == bench?.focusedStatementId,
        'text-gray-700 hover:bg-orange-100': statement.id != bench?.focusedStatementId,
      }"
      @click.prevent="focusStatement(statement)"
      @mousedown.prevent="focusStatement(statement)"
      @keydown.enter.exact.prevent="focusStatementAndGoThere(statement)"
      @keydown.up.exact.prevent="statementsGrid.navigateUp(statement.id, 'name')"
      @keydown.down.exact.prevent="statementsGrid.navigateDown(statement.id, 'name')"
    >
      <span class="">{{ SYMBOL_TYPE_KEYWORD[statement.symbolType] }}</span>
      <span class="">{{ statement.name }}</span>
    </li>
  </ul>
  <div v-else class="my-2 px-3">
    <span class="text-sm text-gray-500">No active file</span>
  </div>
</template>

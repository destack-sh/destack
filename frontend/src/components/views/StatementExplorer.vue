<script lang="ts" setup>
import { useNavigationGrid } from "@/composables/useGrid";
import { StatementType } from "@/gql/graphql";
import { useBenchState, type ViewId } from "@/state/bench";
import {
  orderStatements,
  useCurrentModule,
  useNavigation,
  type InterpStatement,
  getSymbolSubtype,
} from "@/state/module";
import { STATEMENT_TYPE_KEYWORD } from "@/state/type";
import { getStatementIcon } from "@/state/statement";
import { computed, nextTick } from "vue";

const props = defineProps<{ showAllStatements?: boolean }>();
const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
}>();

const runtime = useCurrentModule();
const bench = useBenchState();
const nav = useNavigation();

const orderedStatements = computed(() => {
  if (bench.focusedFileId == null) {
    return undefined;
  }
  const statements = Object.values(runtime.idx.value?.statementsById ?? {}).filter(
    (s) => s.file.id == bench.focusedFileId && s.type != StatementType.Blank && s.type != StatementType.Text
  );
  return orderStatements(statements);
});

const statementsGrid = useNavigationGrid<"name", HTMLElement>(
  computed(() => ["name"]),
  computed(() => orderedStatements.value?.map((s) => s.statement) ?? []),
  {
    gridNavigateUp: () => emit("navigateUp"),
    gridNavigateDown: () => emit("navigateDown"),
  }
);

function focusStatement(statement: InterpStatement) {
  const focusedViewId = bench.focusedViewId;
  nav.focusStatement(statement);
  bench.focusView(focusedViewId as ViewId); // keep focused view
  nextTick(() => nav.focusStatement(statement));
}

function focusStatementAndGoThere(statement: InterpStatement) {
  nav.focusStatement(statement);
}

function focus(target: "first" | "last" = "first") {
  statementsGrid.focus(target == "first" ? 0 : -1, "name");
}

function blur() {
  statementsGrid.blur();
}

defineExpose({
  count: computed(() => orderedStatements.value?.length),
  focus,
  blur,
});
</script>
<template>
  <ul v-if="orderedStatements != null" role="list" class="flex flex-col text-sm">
    <li
      v-for="ordered in orderedStatements"
      :key="ordered.id"
      :ref="(ref) => statementsGrid.registerColumnRef(ordered.id, 'name', ref)"
      tabindex="-1"
      class="flex flex-row gap-1.5 border border-transparent px-3 py-0.5 text-gray-700 outline-none hover:cursor-pointer hover:bg-orange-100 focus:border-orange-600"
      :class="{
        'text-orange-600': ordered.id == bench?.focusedStatementId,
        'text-gray-700 hover:bg-orange-100': ordered.id != bench?.focusedStatementId,
      }"
      :style="{
        marginLeft: ordered.depth * 8 + 'px',
      }"
      @click.prevent="focusStatement(ordered.statement)"
      @mousedown.prevent="focusStatement(ordered.statement)"
      @keydown.enter.exact.prevent="focusStatementAndGoThere(ordered.statement)"
      @keydown.up.exact.prevent="statementsGrid.navigateUp(ordered.id, 'name')"
      @keydown.down.exact.prevent="statementsGrid.navigateDown(ordered.id, 'name')"
    >
      <span class="text rounded-sm font-mono">
        <component
          :is="getStatementIcon(ordered.statement.type, ordered.statement.rootTypeTag)"
          class="mt-0.5 h-4 w-4"
        />
      </span>
      <span class="">{{ ordered.statement.name }}</span>
    </li>
  </ul>
  <div v-else class="my-2 px-3">
    <span class="text-sm text-gray-500">No active file</span>
  </div>
</template>

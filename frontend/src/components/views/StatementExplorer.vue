<script lang="ts" setup>
import { useNavigationGrid } from "@/composables/useGrid";
import { StatementType } from "@/gql/graphql";
import { useBenchState, type ViewId } from "@/state/bench";
import { orderStatements, useCurrentModule, useNavigation, type InterpStatement } from "@/state/module";
import { getStatementIconSolid } from "@/state/statement";
import { computed, nextTick } from "vue";

const props = defineProps<{ showAllStatements?: boolean }>();
const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
}>();

const module = useCurrentModule();
const bench = useBenchState();
const nav = useNavigation();

const orderedStatements = computed(() => {
  if (bench.focusedFileCk == null) {
    return undefined;
  }
  const statements = Object.values(module.idx.value?.statementsById ?? {}).filter(
    (s) => s.file.id == bench.focusedFileId && s.type != StatementType.Blank
  );
  return orderStatements(statements);
});

function getStatementName(statement: {
  id: string;
  type: StatementType;
  name?: string | null;
  referenceCk?: string | null;
}): string | null | undefined {
  if (statement.type == StatementType.Reference) {
    return module.statementOf(statement.referenceCk ?? "")?.name ?? (statement.referenceCk == null ? "..." : "???");
  } else {
    return statement.name;
  }
}

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
      v-for="o in orderedStatements"
      :key="o.id"
      :ref="(ref) => statementsGrid.registerColumnRef(o.id, 'name', ref as HTMLElement)"
      tabindex="-1"
      class="flex max-w-full flex-row gap-1.5 border border-transparent px-3 py-0.5 text-gray-700 outline-none hover:cursor-pointer hover:bg-orange-100 focus:border-orange-600"
      :class="{
        'text-orange-600': o.ck == bench?.focusedStatementCk,
        'text-gray-700 hover:bg-orange-100': o.ck != bench?.focusedStatementCk,
        'mt-1 text-2xl': o.statement.type == StatementType.Text && o.statement.headingLevel == 1,
        'mt-0.5 text-xl': o.statement.type == StatementType.Text && o.statement.headingLevel == 2,
        'text-lg': o.statement.type == StatementType.Text && o.statement.headingLevel == 3,
      }"
      :style="{
        marginLeft: o.depth * 8 + 'px',
      }"
      @click.prevent="focusStatement(o.statement)"
      @mousedown.prevent="focusStatement(o.statement)"
      @keydown.enter.exact.prevent="focusStatementAndGoThere(o.statement)"
      @keydown.up.exact.prevent="statementsGrid.navigateUp(o.id, 'name')"
      @keydown.down.exact.prevent="statementsGrid.navigateDown(o.id, 'name')"
    >
      <!-- Hide icon for text headings -->
      <span
        class="text rounded-sm font-mono"
        v-if="!(o.statement.type == StatementType.Text && o.statement.headingLevel != null)"
      >
        <component
          :is="getStatementIconSolid(o.statement.type, o.statement.tag)"
          class="mt-0.5 h-4 w-4"
          :class="[o.id == bench?.focusedStatementId ? 'text-orange-600' : 'text-gray-400']"
        />
      </span>
      <!-- Show text for unnamed statements -->
      <span
        v-if="
          (o.statement.name ?? '').length == 0 &&
          o.statement.text != null &&
          !(o.statement.type == StatementType.Text && (o.statement.headingLevel ?? 0) > 0)
        "
        class="truncate"
        :class="[o.id == bench?.focusedStatementId ? 'text-orange-600' : 'text-gray-400']"
      >
        {{ o.statement.text }}
      </span>
      <!-- Default to proper name -->
      <span class="truncate" v-else>{{ getStatementName(o.statement) ?? "(Unnamed)" }}</span>
    </li>
  </ul>
  <div v-else class="my-2 px-3">
    <span class="text-sm text-gray-500">No active file</span>
  </div>
</template>

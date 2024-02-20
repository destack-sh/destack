<script lang="ts" setup>
import AnnotatedText from "@/components/interfaces/AnnotatedText.vue";
import { useNavigationGrid } from "@/composables/useGrid";
import { StatementType } from "@/gql/graphql";
import { useBenchState, type ViewId } from "@/state/bench";
import { orderStatements, useCurrentModule, useNavigation, type InterpStatement } from "@/state/module";
import { getStatementIconSolid } from "@/state/statement";
import { computed, nextTick } from "vue";

const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
}>();

const module = useCurrentModule();
const bench = useBenchState();
const nav = useNavigation();

const orderedStatements = computed(() => {
  if (bench.focusedFileCk == null) return undefined;
  const statements = Object.values(module.idx.value?.statementsById ?? {}).filter(
    (s) => s.file.id == bench.focusedFileId
  );
  const { ordered, statementsByParentId } = orderStatements(statements);
  // exclude blank statements without children
  return ordered.filter(
    (o) => o.statement.type != StatementType.Blank || (statementsByParentId[o.id]?.length ?? 0) > 0
  );
});
const focusedStatementCk = computed(() => bench.focusedStatementCk);

function getStatementName(
  statement: {
    id: string;
    type: StatementType;
    name?: string | null;
    referenceCk?: string | null;
  },
  defaultName: string
): string | null | undefined {
  let name;
  if (statement.type == StatementType.Reference) {
    name = module.statementOf(statement.referenceCk ?? "")?.name ?? (statement.referenceCk == null ? "..." : "???");
  } else {
    name = statement.name;
  }
  if ((name ?? "")?.trim().length == 0) {
    return defaultName;
  } else {
    return name;
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

const HEADING_TEXT_SIZE: Record<number, string> = {
  1: "text-xl",
  2: "text-lg",
  3: "text-base",
};
const HEADING_MARGIN_TOP: Record<number, string> = {
  1: "mt-1.5",
  2: "mt-1",
  3: "mt-0.5",
};

defineExpose({
  count: computed(() => orderedStatements.value?.length),
  focus,
  blur,
});
</script>
<template>
  <ul v-if="orderedStatements != null" role="list" class="flex flex-col text-sm">
    <li
      v-for="(o, i) in orderedStatements"
      :key="o.id"
      :ref="(ref) => statementsGrid.registerColumnRef(o.id, 'name', ref as HTMLElement)"
      tabindex="-1"
      class="flex max-w-full flex-row border border-transparent px-3 py-0.5 text-gray-700 outline-none hover:cursor-pointer hover:bg-orange-100 focus:border-orange-600"
      :class="[
        o.ck == focusedStatementCk ? 'text-orange-600' : 'text-gray-700 hover:bg-orange-100',
        o.statement.headingLevel != null ? HEADING_TEXT_SIZE[o.statement.headingLevel] + ' -mb-0.5 font-semibold' : '',
        o.statement.headingLevel != null && i > 0 ? HEADING_MARGIN_TOP[o.statement.headingLevel] : '',
      ]"
      :style="{
        marginLeft: o.renderedDepth * 8 + 'px',
      }"
      @click.prevent="focusStatement(o.statement)"
      @mousedown.prevent="focusStatement(o.statement)"
      @keydown.enter.exact.prevent="focusStatementAndGoThere(o.statement)"
      @keydown.up.exact.prevent="statementsGrid.navigateUp(o.id, 'name')"
      @keydown.down.exact.prevent="statementsGrid.navigateDown(o.id, 'name')"
    >
      <!-- Hide icon for text headings -->
      <span
        v-if="
          !(o.statement.type == StatementType.Text && (o.statement.headingLevel ?? 0) > 0) &&
          o.statement.type != StatementType.Blank &&
          o.statement.type != StatementType.Group
        "
        class="mr-1.5 rounded-sm font-mono"
      >
        <component
          :is="getStatementIconSolid(o.statement.type)"
          class="mt-0.5 h-4 w-4"
          :class="[o.ck == bench?.focusedStatementCk ? 'text-orange-600' : 'text-gray-400']"
        />
      </span>
      <!-- 'Name' -->
      <!-- Blank statement is only shown if it has descendants -->
      <span v-if="o.statement.type == StatementType.Blank" class="text-gray-400">(Blank)</span>
      <!-- Show text for unnamed statements -->
      <AnnotatedText
        v-else-if="(o.statement.name ?? '').length == 0 && o.statement.text != null"
        :model-value="o.statement.text"
        readonly
        minimal-mentions
        class="max-w-full truncate"
        :class="[
          o.ck == bench?.focusedStatementCk
            ? 'text-orange-600'
            : o.statement.headingLevel != null
            ? 'text-gray-700'
            : 'text-gray-400',
          ,
        ]"
      />
      <!-- Default to proper name -->
      <span class="truncate" v-else>{{ getStatementName(o.statement, "(Unnamed)") }}</span>
    </li>
  </ul>
  <div v-else class="my-2 px-3">
    <span class="text-sm text-gray-500">No active file</span>
  </div>
</template>

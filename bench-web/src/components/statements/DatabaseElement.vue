<script lang="ts" setup>
import { useActiveScroll } from "@/composables/useScroll";
import { SortOp, type Sort, TypeTag } from "@/gql/graphql";
import {
  usePanelContext,
  useElementPanelSettings,
  type StatementAction,
  useBenchState,
  EditDatabasePanel,
} from "@/state/bench";
import { useCurrentModule, type Field, type Record, type NodeBase } from "@/state/module";
import { useFields, type DatabaseStatementProperties } from "@/state/statement";
import { IS_DEBUG, IS_LOCALHOST } from "@/utils/globals";
import {
  ArrowPathIcon,
  PlusIcon,
  ChevronDoubleDownIcon,
  ChevronDoubleUpIcon,
  SquaresPlusIcon,
  ArrowsPointingOutIcon,
} from "@heroicons/vue/24/outline";
import { useMouseInElement } from "@vueuse/core";
import { computed, nextTick, ref, watch, type Ref, onMounted, toRef } from "vue";
import type { StatementEmit, StatementProps } from "@/components/statements";
import { getDefaultConditional, useDatabaseCombinedSearch } from "@/state/database";
import DatabaseTile from "@/components/tiles/DatabaseTile.vue";
import CreateFieldInterface from "@/components/interfaces/CreateFieldInterface.vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import SortSetTile from "@/components/tiles/SortSetTile.vue";
import ConditionalSetTile from "@/components/tiles/ConditionalSetTile.vue";
import QuickSearchTile from "@/components/tiles/QuickSearchTile.vue";
import ViewPaginationTile from "@/components/tiles/ViewPaginationTile.vue";

const PAGE_SIZE = 10;

const props =
  defineProps<Pick<StatementProps, "statement" | "focused" | "readonly" | "editing" | "xoffset" | "visible">>();
const emit = defineEmits<StatementEmit>();

const bench = useBenchState();
const module = useCurrentModule();
const panel = usePanelContext();

const gridRef: Ref<HTMLDivElement | null> = ref(null);
const innerGridRef: Ref<HTMLDivElement | null> = ref(null);
const loadMoreRef: Ref<HTMLButtonElement | null> = ref(null);
const addRecordRef: Ref<HTMLButtonElement | null> = ref(null);
const createFieldRef: Ref<InstanceType<typeof CreateFieldInterface> | null> = ref(null);
const contentRef: Ref<InstanceType<typeof DatabaseTile> | null> = ref(null);

const properties = useElementPanelSettings<DatabaseStatementProperties>(toRef(props, "statement"), {
  inlineQuery: undefined,
  wrapColumns: false,
});
const fields = useFields(toRef(props, "statement"));
// reset inline query to undefined if it's empty on load
onMounted(() => {
  if ((properties.inlineQuery ?? "").trim().length == 0) {
    properties.inlineQuery = undefined;
  }
});

const { combinedQuery, queryEngine } = useDatabaseCombinedSearch(
  fields,
  computed(() => properties.inlineQuery),
  computed(() => properties.filters ?? [])
);
const after: Ref<string | undefined> = ref(undefined);

function addSort(field: Field, order: SortOp) {
  if (properties.sorts == null) properties.sorts = [];
  // replace or append sort
  const oldIndex = properties.sorts.findIndex((s) => s.field == field.ck);
  if (oldIndex >= 0) {
    properties.sorts.splice(oldIndex, 1, { field: field.ck, order });
  } else {
    properties.sorts.push({ field: field.ck, order });
  }
}
function removeSort(sort: { field: string }) {
  properties.sorts = properties.sorts?.filter((s) => !s.field.includes(sort.field));
}
const sort: Ref<Sort[] | null> = computed(() => {
  if (properties.sorts == null || properties.sorts.length == 0) return null;
  return properties.sorts;
});

function addDefaultConditional(field: Field) {
  if (properties.filters == null) properties.filters = [];
  properties.filters.push(getDefaultConditional(field));
}

// navigation
useActiveScroll(gridRef);

function openAsDatabasePanel() {
  const panel = bench.openEditDatabase(props.statement as NodeBase, { focus: true }) as EditDatabasePanel;
  panel.sorts = properties.sorts;
  panel.filters = properties.filters;
  panel.inlineQuery = properties.inlineQuery;
}

// drag & drop
const position = useMouseInElement(gridRef);
const gridOffsetX: Ref<number> = computed(() => {
  if (panel.size.value.width > panel.panel.value.contentWidthWithMargin) {
    return (panel.size.value.width - panel.panel.value.contentWidth) / 2;
  } else {
    return panel.panel.value.contentMarginX;
  }
});

// actions

const actions = computed(() => {
  const actions: StatementAction[] = [];
  if (IS_LOCALHOST || IS_DEBUG) {
    actions.push({
      label: "Reload view",
      icon: ArrowPathIcon,
      active: contentRef.value?.loading,
      action: () => contentRef.value?.refetch(),
    });
  }
  actions.push({
    label: "Add field",
    groupId: "edit",
    icon: SquaresPlusIcon,
    disabled: props.readonly,
    action: () => {
      createFieldRef.value?.show();
    },
  });
  actions.push({
    label: "Add record",
    groupId: "edit",
    disabled: props.readonly,
    icon: PlusIcon,
    action: () => {
      contentRef.value?.insertRecordAtEnd();
    },
  });
  actions.push({
    label: "View all",
    groupId: "nav",
    icon: ArrowsPointingOutIcon,
    action: () => {
      openAsDatabasePanel();
    },
  });
  actions.push({
    label: properties.wrapColumns ? "Unwrap columns" : "Wrap columns",
    groupId: "nav",
    icon: properties.wrapColumns ? ChevronDoubleUpIcon : ChevronDoubleDownIcon,
    action: () => (properties.wrapColumns = !properties.wrapColumns),
    hideInline: true,
  });
  return actions;
});

defineExpose({
  focus: (position: "first" | "last" = "first") => {
    if (position == "first") {
      if ((contentRef?.value?.recordsInView.length ?? 0) > 0) {
        contentRef.value?.focus("first");
      } else {
        addRecordRef.value?.focus();
      }
    } else {
      contentRef.value?.focus("last");
    }
  },
  blur: () => {
    addRecordRef.value?.blur();
    contentRef.value?.blur();
  },
  // prevent outer drag and drop while inside grid
  capturingDrag: computed(() => !position.isOutside.value),
  loading: computed(() => contentRef.value?.loading ?? false),
  actions,
});
</script>
<template>
  <div>
    <!-- Views: sorts/filters/pagination -->
    <div class="-mx-0.5 mb-1 flex flex-row flex-wrap items-center gap-2">
      <QuickSearchTile
        :modelValue="properties.inlineQuery"
        @update:modelValue="(properties.inlineQuery = $event), (after = undefined)"
        placeholder="Search..."
      />
      <ConditionalSetTile
        :fields="fields.allFields.value"
        :model-value="properties.filters"
        @update:model-value="(properties.filters = $event), (after = undefined)"
      />
      <SortSetTile
        :fields="fields.allFields.value"
        :model-value="properties.sorts"
        @update:model-value="(properties.sorts = $event), (after = undefined)"
      />
      <ViewPaginationTile
        class="ml-auto"
        :page-info="contentRef?.pageInfo"
        :total-count="contentRef?.totalCount"
        :query="combinedQuery"
        :sort="sort"
        v-model="after"
      />
    </div>
    <!-- Table (in table form but manually sized) -->
    <!-- Wrapper to contain any scrolling -->
    <div
      ref="gridRef"
      class="overflow-x-auto"
      :style="{
        'margin-left': -gridOffsetX + 'px',
        'margin-right': -gridOffsetX + 'px',
        'padding-left': gridOffsetX + 'px',
        'padding-right': gridOffsetX + 'px',
        'max-width': panel.size.value.width + 'px',
      }"
    >
      <!-- Inner grid -->
      <div ref="innerGridRef" class="-mx-1 flex min-w-fit flex-col">
        <CreateFieldInterface
          ref="createFieldRef"
          :title="'New field'"
          :ref-types="[TypeTag.Enum, TypeTag.Struct]"
          @select="contentRef?.createNewField($event)"
        />
        <DatabaseTile
          ref="contentRef"
          :statement="props.statement"
          :query="combinedQuery"
          :query-engine="queryEngine"
          :sort="sort"
          :after="after"
          :readonly="props.readonly"
          :wrap="properties.wrapColumns"
          :editing="props.editing"
          :target-min-width="
            Math.min(panel.size.value.width - panel.panel.value.contentMarginX * 2, panel.panel.value.contentWidth) -
            props.xoffset -
            8
          "
          :page-size="PAGE_SIZE"
          show-record-action-popover
          @navigate-up="emit('navigateUp')"
          @navigate-down="loadMoreRef != null ? loadMoreRef.focus() : emit('navigateDown')"
          @create-field="createFieldRef?.show()"
          @add-sort="({ field, order }) => addSort(field, order ?? SortOp.Ascending)"
          @add-filter="({ field }) => addDefaultConditional(field)"
          @open-actions="emit('openActions')"
        />
        <!-- Load more/loading/go to big database view -->
        <div
          v-if="contentRef?.loading"
          class="flex flex-row rounded-sm border-b border-orange-900/[12%] group-focus-within/statement:text-gray-400"
          :style="{ height: 32 + 'px' }"
          @keydown.up.exact.prevent="contentRef?.focus('last')"
          @keydown.down.exact.prevent="emit('navigateDown')"
        >
          <!-- Load more abandoned because of performance issues -->
          <!-- Go to big database view -->
          <button
            ref="openPanelViewRef"
            class="flex flex-1 flex-row items-center px-1 py-1 text-gray-300 outline-none transition duration-75 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100"
            @click.stop="openAsDatabasePanel()"
          >
            <BusySpinnerIcon class="mr-1 h-4 w-4 animate-spin" />
            Loading
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

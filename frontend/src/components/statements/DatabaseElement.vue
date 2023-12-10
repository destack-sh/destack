<script lang="ts" setup>
import { useActiveScroll } from "@/composables/useScroll";
import { SortOp, type Sort, TypeTag } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
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
  XMarkIcon,
  SquaresPlusIcon,
  ArrowUpRightIcon,
  ArrowsPointingOutIcon,
} from "@heroicons/vue/24/outline";
import { useApolloClient, useQuery } from "@vue/apollo-composable";
import { useMouseInElement } from "@vueuse/core";
import { computed, nextTick, ref, watch, type Ref, onMounted, toRef } from "vue";
import type { StatementEmit, StatementProps } from "@/components/statements";
import { useDatabaseInlineSearch } from "@/state/database";
import { humanizeNumber } from "@/composables/useNow";
import DatabaseTile from "@/components/tiles/DatabaseTile.vue";
import CreateFieldInterface from "@/components/interfaces/CreateFieldInterface.vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import SortSetTile from "@/components/tiles/SortSetTile.vue";
import ConditionalSetTile from "@/components/tiles/ConditionalSetTile.vue";
import ViewPaginationTile from "@/components/tiles/ViewPaginationTile.vue";

const PAGE_SIZE = 10;

const props =
  defineProps<Pick<StatementProps, "statement" | "focused" | "readonly" | "editing" | "xoffset" | "visible">>();
const emit = defineEmits<StatementEmit>();

const bench = useBenchState();
const module = useCurrentModule();
const panel = usePanelContext();
const appearance = useAppearance();
const client = useApolloClient();

const gridRef: Ref<HTMLDivElement | null> = ref(null);
const innerGridRef: Ref<HTMLDivElement | null> = ref(null);
const loadMoreRef: Ref<HTMLButtonElement | null> = ref(null);
const addRecordRef: Ref<HTMLButtonElement | null> = ref(null);
const createFieldRef: Ref<InstanceType<typeof CreateFieldInterface> | null> = ref(null);
const databaseTileRef: Ref<InstanceType<typeof DatabaseTile> | null> = ref(null);

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

const { inlineQuery, queryEngine } = useDatabaseInlineSearch(fields, toRef(properties, "inlineQuery"));

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
      active: databaseTileRef.value?.loading,
      action: () => databaseTileRef.value?.refetch(),
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
      databaseTileRef.value?.insertRecordAtEnd();
    },
  });
  actions.push({
    label: "View all",
    groupId: "nav",
    icon: ArrowsPointingOutIcon,
    action: () => {
      openAsDatabasePanel();
    },
    hideInline: true,
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
      if ((databaseTileRef?.value?.recordsInView.length ?? 0) > 0) {
        databaseTileRef.value?.focus("first");
      } else {
        addRecordRef.value?.focus();
      }
    } else {
      databaseTileRef.value?.focus("last");
    }
  },
  blur: () => {
    addRecordRef.value?.blur();
    databaseTileRef.value?.blur();
  },
  // prevent outer drag and drop while inside grid
  capturingDrag: computed(() => !position.isOutside.value),
  loading: computed(() => databaseTileRef.value?.loading ?? false),
  actions,
});
</script>
<template>
  <div>
    <!-- Views: sorts/filters/pagination -->
    <div class="-mx-0.5 mb-1 flex flex-row flex-wrap items-center gap-2">
      <ConditionalSetTile
        :fields="fields.allFields.value"
        :model-value="properties.filters"
        @update:model-value="properties.filters = $event"
      />
      <SortSetTile
        :fields="fields.allFields.value"
        :model-value="properties.sorts"
        @update:model-value="properties.sorts = $event"
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
          @select="databaseTileRef?.createNewField($event)"
        />
        <DatabaseTile
          ref="databaseTileRef"
          :statement="props.statement"
          :query="inlineQuery"
          :query-engine="queryEngine"
          :sort="sort"
          :readonly="props.readonly"
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
        />
        <!-- Load more/loading/go to big database view -->
        <div
          class="flex flex-row rounded-sm border-b border-orange-900/[12%] group-focus-within/statement:text-gray-400"
          :style="{ height: 32 + 'px' }"
          @keydown.up.exact.prevent="databaseTileRef?.focus('last')"
          @keydown.down.exact.prevent="emit('navigateDown')"
        >
          <!-- Load more abandoned because of performance issues -->
          <!-- Go to big database view -->
          <button
            ref="openPanelViewRef"
            class="flex flex-1 flex-row items-center px-1 py-1 text-gray-300 outline-none transition duration-75 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100"
            @click.stop="openAsDatabasePanel()"
          >
            <template v-if="databaseTileRef?.loading">
              <BusySpinnerIcon class="mr-1 h-4 w-4 animate-spin" />
              Loading
            </template>
            <template v-else>
              <ArrowUpRightIcon class="mr-1 h-4 w-4" />
              View all ({{ humanizeNumber(databaseTileRef?.totalCount ?? 0) }} records)
            </template>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import PanelHeader from "@/components/panels/PanelHeader.vue";
import PanelStatusNotice from "@/components/panels/PanelStatusNotice.vue";
import DatabaseTile from "@/components/tiles/DatabaseTile.vue";
import { useActiveScroll } from "@/composables/useScroll";
import { useBenchState, type PanelContext, EditDatabasePanel } from "@/state/bench";
import { type Statement, useCurrentModule, type NodeBase, type Field } from "@/state/module";
import { computed, ref, watch, type Ref, watchEffect, toRef } from "vue";
import SortSetTile from "@/components/tiles/SortSetTile.vue";
import ConditionalSetTile from "@/components/tiles/ConditionalSetTile.vue";
import { useFields } from "@/state/statement";
import { SortOp } from "@/gql/graphql";
import { getDefaultConditional, useDatabaseCombinedSearch } from "@/state/database";
import QuickSearchTile from "@/components/tiles/QuickSearchTile.vue";
import ViewPaginationTile from "@/components/tiles/ViewPaginationTile.vue";
import { ChevronDoubleDownIcon } from "@heroicons/vue/24/outline";

const PAGE_SIZE = 50;

const props = defineProps<{ panel: PanelContext<EditDatabasePanel>; focused: boolean }>();
const emit = defineEmits<{ (e: "close"): void }>();
const bench = useBenchState();
const module = useCurrentModule();
const panel = computed(() => props.panel.panel.value);
const statement = computed(() => module.statementOf(props.panel.panel.value.statementCk));
const path = computed(() => module.nodePathOf(props.panel.panel.value.statementCk));
const fields = useFields(statement);

const { combinedQuery } = useDatabaseCombinedSearch(
  fields,
  computed(() => panel.value.inlineQuery),
  computed(() => panel.value.filters ?? [])
);
const after: Ref<string | undefined> = ref(undefined);

const contentRef = ref<InstanceType<typeof DatabaseTile> | null>(null);

// mark as 'loading' until loaded once
const loading: Ref<boolean> = ref(true);
watchEffect(() => {
  if (!module.loading.value && contentRef.value != null && !contentRef.value.loading) {
    loading.value = false;
  }
});

useActiveScroll(computed(() => contentRef.value?.$el));

function addSort(field: Field, order: SortOp) {
  const value = [...(panel.value.sorts ?? [])];
  value.push({ field: field.ck, order });
  panel.value.sorts = value;
}

function addDefaultConditional(field: Field) {
  const value = [...(panel.value.filters ?? [])];
  value.push(getDefaultConditional(field));
  panel.value.filters = value;
}

// sync name/path into editor
watch(
  () => [statement.value, module.idx.value],
  () => {
    if (statement.value == null || module.idx.value == null) return;
    panel.value.updatePath({ ...statement.value }, module.idx.value);
  }
);
</script>
<template>
  <!-- Database container -->
  <div class="relative h-full bg-white">
    <!-- Preamble -->
    <PanelHeader
      class="border-b border-orange-900/[12%] bg-white"
      :thing="statement"
      :actions="[]"
      :editing="panel.editing"
      :path="path ?? []"
      :self="(path?.length ?? 0) - 1"
      :subpath="statement?.name"
      @focus="(n) => (n.__typename == 'File' ? bench.openEditFile(n as NodeBase, { focus: true}) : false)"
    />
    <!-- Loading / status -->
    <div
      v-if="loading"
      class="flex h-full w-full flex-col items-center justify-center"
      :style="{
        width: props.panel.size.value?.width + 'px',
        height: props.panel.size.value?.height + 'px',
      }"
    >
      <BusySpinnerIcon class="mx-auto h-8 w-8 animate-spin text-gray-700" />
    </div>
    <PanelStatusNotice :thing="statement" name="file" :loading="loading" />

    <!-- Header (search/views/pagination/create) -->
    <div class="flex w-full flex-row flex-wrap gap-1.5 pb-2 pl-6 pr-4 pt-8 text-sm">
      <QuickSearchTile
        :modelValue="panel.inlineQuery"
        @update:modelValue="(panel.inlineQuery = $event), (after = undefined)"
        placeholder="Search..."
      />
      <ConditionalSetTile
        :fields="fields.allFields.value"
        :model-value="panel.filters"
        @update:model-value="(panel.filters = $event), (after = undefined)"
      />
      <SortSetTile
        :fields="fields.allFields.value"
        :model-value="panel.sorts"
        @update:model-value="(panel.sorts = $event), (after = undefined)"
      />
      <div class="ml-auto flex flex-row gap-1.5">
        <div v-if="panel.selectedElementIds.length > 0" class="flex flex-row rounded-xl px-1 ring-1 ring-amber-600/60">
          <button class="rounded-xl p-1 text-amber-600 hover:bg-amber-100" @click="panel.clearSelection()">
            {{ panel.selectedElementIds.length }} records
          </button>
          <!-- TODO @UX: support batch ops for records once we have :BE-114 -->
        </div>
        <button
          class="flex flex-row items-center rounded-sm p-1 text-gray-400 hover:bg-orange-100 hover:text-gray-700"
          @click="panel.wrap = !panel.wrap"
        >
          <ChevronDoubleDownIcon
            class="h-4 w-4 transform transition-transform duration-75"
            :class="panel.wrap ? 'rotate-0' : 'rotate-180'"
          />
          <span class="ml-0.5">{{ panel.wrap ? "Wrapped" : "Compact" }}</span>
        </button>
        <ViewPaginationTile
          :page-info="contentRef?.pageInfo"
          :total-count="contentRef?.totalCount ?? undefined"
          v-model="after"
          :query="combinedQuery"
          :sort="panel.sorts"
        />
      </div>
    </div>

    <!-- Content -->
    <DatabaseTile
      v-if="statement != null"
      ref="contentRef"
      class="h-full w-full overflow-x-auto overflow-y-scroll pb-20 pl-6 pr-0"
      :style="{
        maxWidth: props.panel.size.value?.width + 'px',
      }"
      :statement="(statement as Statement)"
      :target-min-width="props.panel.size.value?.width - 20 ?? 0"
      :page-size="PAGE_SIZE"
      :padding-left="8 /* for record actions since this is full panel */"
      :query="combinedQuery"
      :sort="panel.sorts"
      :after="after"
      :wrap="panel.wrap"
      selectable
      show-record-action-popover
      sticky-header
      v-model:selected-record-ids="panel.selectedElementIds"
      @add-sort="({ field, order }) => addSort(field, order ?? SortOp.Ascending)"
      @add-filter="({ field }) => addDefaultConditional(field)"
    />
  </div>
</template>

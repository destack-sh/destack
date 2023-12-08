<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import PanelHeader from "@/components/panels/PanelHeader.vue";
import PanelStatusNotice from "@/components/panels/PanelStatusNotice.vue";
import DatabaseTile from "@/components/tiles/DatabaseTile.vue";
import { useActiveScroll } from "@/composables/useScroll";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type PanelContext, EditDatabasePanel } from "@/state/bench";
import { type Statement, useCurrentModule, type NodeBase } from "@/state/module";
import { computed, ref, watch } from "vue";

const PAGE_SIZE = 50;

const props = defineProps<{ panel: PanelContext<EditDatabasePanel>; focused: boolean }>();
const emit = defineEmits<{ (e: "close"): void }>();
const bench = useBenchState();
const module = useCurrentModule();
const panel = computed(() => props.panel.panel.value);
const appearance = useAppearance();
const statement = computed(() => module.statementOf(props.panel.panel.value.statementCk));
const path = computed(() => module.nodePathOf(props.panel.panel.value.statementCk));

const contentRef = ref<InstanceType<typeof DatabaseTile> | null>(null);
useActiveScroll(computed(() => contentRef.value?.$el));

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
      v-if="module.loading.value"
      class="flex h-full w-full flex-col items-center justify-center"
      :style="{
        width: props.panel.size.value?.width + 'px',
        height: props.panel.size.value?.height + 'px',
      }"
    >
      <BusySpinnerIcon class="mx-auto h-8 w-8 animate-spin text-gray-700" />
    </div>
    <PanelStatusNotice :thing="statement" name="file" :loading="module.loading.value" />

    <!-- Header (search/views/pagination/create) -->
    <div class="w-full px-2 pb-1.5 pt-8 text-sm">
      <span>TODO search nocheckin</span>
    </div>

    <!-- Content -->
    <DatabaseTile
      v-if="statement != null"
      ref="contentRef"
      class="h-full w-full overflow-x-auto overflow-y-auto"
      :style="{
        maxWidth: props.panel.size.value?.width + 'px',
      }"
      :statement="(statement as Statement)"
      :target-min-width="props.panel.size.value?.width ?? 0"
      :page-size="PAGE_SIZE"
      :padding-left="8 /* for record actions since this is full panel */"
      selectable
    />
  </div>
</template>

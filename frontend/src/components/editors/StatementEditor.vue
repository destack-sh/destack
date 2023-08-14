<script lang="ts" setup>
import EditedThingBanner from "@/components/editors/EditedThingBanner.vue";
import Statement from "@/components/editors/Statement.vue";
import FixedInlineHeader from "@/components/editors/FixedInlineHeader.vue";
import { graphql, useFragment } from "@/gql";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type PanelContext, type StatementEditor } from "@/state/bench";
import { FileHeaderType, StatementContentType } from "@/state/fragments";
import { useCurrentModule } from "@/state/module";
import { useQuery } from "@vue/apollo-composable";
import { computed, ref, watch, watchEffect } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";

const props = defineProps<{ panel: PanelContext<StatementEditor>; focused: boolean }>();
const emit = defineEmits<{ (e: "close"): void }>();
const bench = useBenchState();
const module = useCurrentModule();
const appearance = useAppearance();
const panel = computed(() => props.panel.panel.value);

// statement state

const { result: statementResult, loading: statementLoading } = useQuery(
  graphql(/* GraphQL */ `
    query statementContentById($statementId: GlobalID!) {
      statement(id: $statementId) {
        id
        projectVersion {
          id
        }
        file {
          ...FileHeader
        }
        deletedAt
        ...StatementContent
      }
    }
  `),
  () => ({
    statementId: props.panel.panel.value.statementId,
  })
);

const statement = computed(() => useFragment(StatementContentType, statementResult.value?.statement) ?? undefined);
const file = computed(() => useFragment(FileHeaderType, statementResult.value?.statement?.file) ?? undefined);
const statementComponentRef = ref<InstanceType<typeof Statement> | null>(null);
const statementComponentLoaded = ref(false);
watchEffect(() => {
  if (statementComponentLoaded.value || statementComponentRef.value == null) return;
  if (statementComponentRef.value?.loading === false) {
    statementComponentLoaded.value = true;
  }
});

// sync name/path into editor
watch(
  () => [statement.value?.name, statement.value == null || module.fileOf(statement.value)],
  () => {
    if (statement.value != null && module.idx.value != null && module.fileOf(statement.value) != null) {
      panel.value.updatePath(statement.value, module.idx.value);
    }
  }
);
</script>
<template>
  <div class="overflow-x-hidden bg-white">
    <FixedInlineHeader
      :thing="statement"
      :actions="statementComponentRef?.allActions ?? []"
      :editing="false /* not sure */"
      :readonly="bench.readonly"
      :path="panel.path"
    />
    <EditedThingBanner :thing="statementResult?.statement" :is-loading="statementLoading" name="statement" />
    <!-- Loading -->
    <div
      v-if="statementLoading || !statementComponentLoaded"
      class="flex h-full w-full flex-col items-center justify-center"
      :style="{
        width: props.panel.size.value?.width + 'px',
        height: props.panel.size.value?.height + 'px',
      }"
    >
      <BusySpinnerIcon class="mx-auto h-8 w-8 animate-spin text-gray-700" />
    </div>
    <!-- Statement content -->
    <div class="mt-8 flex flex-col bg-white" v-if="statement != null && file != null" v-show="statementComponentLoaded">
      <!-- Non-clickable invisible overlay if deleted -->
      <div v-if="statement?.deletedAt != null" class="absolute inset-0 z-20 flex justify-center opacity-100" />
      <!-- Editor inline header -->
      <!-- Title -->
      <!-- TODO @UX: parse and enrich statement editor title & use larger space for statements in standalone editor -->
      <!-- Statement -->
      <Statement
        ref="statementComponentRef"
        class="relative mx-auto w-full justify-between pb-10 pt-4"
        :class="appearance.baseClass"
        :style="{
          'max-width': panel.contentWidth + panel.contentMarginX * 2 + 'px',
          paddingLeft: `${panel.contentMarginX}px`,
          paddingRight: `${panel.contentMarginX}px`,
        }"
        :file="file"
        :statement="statement"
        :readonly="bench.readonly"
        :depth="0"
        :ancestors="[]"
        standalone
        :shown="statementComponentLoaded"
      />
    </div>
  </div>
</template>

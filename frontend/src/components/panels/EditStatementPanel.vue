<script lang="ts" setup>
import PanelStatusNotice from "@/components/panels/PanelStatusNotice.vue";
import Statement from "@/components/panels/Statement.vue";
import PanelHeader from "@/components/panels/PanelHeader.vue";
import { graphql } from "@/gql";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type PanelContext, type EditStatementPanel, type FileHeader } from "@/state/bench";
import { useCurrentModule, type Statement as StatementType, getNodeIdFromCk } from "@/state/module";
import { useQuery } from "@vue/apollo-composable";
import { computed, ref, watch, watchEffect } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";

const props = defineProps<{ panel: PanelContext<EditStatementPanel>; focused: boolean }>();
const emit = defineEmits<{ (e: "close"): void }>();
const bench = useBenchState();
const module = useCurrentModule();
const appearance = useAppearance();
const panel = computed(() => props.panel.panel.value);

// statement state

const {
  result: statementResult,
  loading: statementLoading,
  error: statementError,
} = useQuery(
  graphql(/* GraphQL */ `
    query statementContentById($statementId: GlobalID!) {
      statement(id: $statementId) {
        id
        projectVersion {
          id
        }
        parent {
          id
        }
        deletedAt
        ...StatementContent
      }
    }
  `),
  () => ({
    statementId: getNodeIdFromCk(bench.projectVersionId as string, panel.value.statementCk, "Statement"),
  })
);

const statement = computed(() => statementResult.value?.statement as StatementType);
const file = computed(() => module.fileOf(statement.value.id) as FileHeader | undefined);
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
  () => [statement.value?.name, statement.value == null || module.fileOf(panel.value.statementCk)],
  () => {
    if (statement.value != null && module.idx.value != null && module.fileOf(panel.value.statementCk) != null) {
      panel.value.updatePath(statement.value, module.idx.value);
    }
  }
);

const statementPath = computed(() => module.nodePathOf(statement.value?.id));
</script>
<template>
  <div class="overflow-x-hidden bg-white">
    <PanelHeader
      :thing="statement"
      :actions="statementComponentRef?.allActions ?? []"
      :editing="false /* not sure */"
      :readonly="bench.readonly"
      :path="statementPath ?? []"
      :self="(statementPath?.length ?? 0) - 1"
      hide-wide-toggle
    />
    <PanelStatusNotice
      :thing="statementResult?.statement"
      :loading="statementLoading"
      :error="statementError"
      name="statement"
    />
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
        class="relative mx-auto w-full justify-between"
        :class="appearance.baseClass"
        :style="{
          'max-width': panel.contentWidth + panel.contentMarginX * 2 + 'px',
          paddingLeft: `${panel.contentMarginX}px`,
          paddingRight: `${panel.contentMarginX}px`,
        }"
        :file="(file as FileHeader)"
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

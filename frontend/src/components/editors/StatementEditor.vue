<script lang="ts" setup>
import EditedThingBanner from "@/components/editors/EditedThingBanner.vue";
import Statement from "@/components/editors/Statement.vue";
import TitleBanner from "@/components/editors/TitleBanner.vue";
import FixedInlineHeader from "@/components/editors/FixedInlineHeader.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql, useFragment } from "@/gql";
import { useActions } from "@/state/actions";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type EditorContext, type StatementEditor } from "@/state/bench";
import { FileHeaderType, StatementContentType } from "@/state/fragments";
import { useCurrentModule } from "@/state/module";
import { useOperations } from "@/state/operations";
import { useQuery } from "@vue/apollo-composable";
import { computed, ref, watch, watchEffect } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";

const props = defineProps<{ editor: EditorContext<StatementEditor>; focused: boolean }>();
const emit = defineEmits<{ (e: "close"): void }>();
const bench = useBenchState();
const module = useCurrentModule();
const appearance = useAppearance();
const actions = useActions();
const editor = computed(() => props.editor.editor.value);
const now = useTimeFromNow();
const ops = useOperations();

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
    statementId: props.editor.editor.value.statementId,
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
      editor.value.updatePath(statement.value, module.idx.value);
    }
  }
);
</script>
<template>
  <div class="overflow-x-hidden bg-white">
    <FixedInlineHeader
      :thing="statement"
      :actions="[]"
      :editing="false /* not sure */"
      :readonly="bench.readonly"
      :path="editor.path"
    />
    <EditedThingBanner :thing="statementResult?.statement" :is-loading="statementLoading" name="statement" />
    <!-- Loading -->
    <div
      v-if="statementLoading || !statementComponentLoaded"
      class="flex h-full w-full flex-col items-center justify-center"
      :style="{
        width: props.editor.size.value?.width + 'px',
        height: props.editor.size.value?.height + 'px',
      }"
    >
      <BusySpinnerIcon class="mx-auto h-8 w-8 animate-spin text-gray-700" />
    </div>
    <!-- Statement content -->
    <div class="flex flex-col bg-white" v-if="statement" v-show="statementComponentLoaded">
      <!-- Non-clickable invisible overlay if deleted -->
      <div v-if="statement?.deletedAt != null" class="absolute inset-0 z-20 flex justify-center opacity-100" />
      <!-- Editor inline header -->
      <!-- Title -->
      <!-- TODO @UX: parse and enrich statement editor title & prettify statements in this view -->
      <!-- also see how we currently assume unnested statements and parse editor paths -->
      <TitleBanner
        class="relative mx-auto w-full justify-between pt-14"
        :class="appearance.baseClass"
        :style="{
          'max-width': editor.contentWidth + editor.contentMarginX * 2 + 'px',
          paddingLeft: `${editor.contentMarginX + 6}px`, // + for :StatementPadding
          paddingRight: `${editor.contentMarginX + 6}px`,
        }"
        :readonly="true"
        :thing="statement"
        :model-value="statement.name"
        :actions="[]"
      />
      <!-- Statement -->
      <Statement
        ref="statementComponentRef"
        class="relative mx-auto w-full justify-between pb-10 pt-4"
        :class="appearance.baseClass"
        :style="{
          'max-width': editor.contentWidth + editor.contentMarginX * 2 + 'px',
          paddingLeft: `${editor.contentMarginX}px`,
          paddingRight: `${editor.contentMarginX}px`,
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

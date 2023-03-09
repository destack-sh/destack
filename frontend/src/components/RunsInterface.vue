<script lang="ts" setup>
import RunsTable from "@/components/basic/RunsTable.vue";
import { humanizeNumber } from "@/composables/useNow";
import { useAppearance } from "@/state/appearance";
import { useEditorState } from "@/state/editor";
import { ref } from "vue";

const props = defineProps<{ focused: boolean }>();
const editor = useEditorState();
const appearance = useAppearance();

const runsTableRef = ref<InstanceType<typeof RunsTable> | null>(null);

function openRunsEditor() {
  const e = editor.openRuns({ create: true });
  editor.focusEditor(e);
}
</script>
<template>
  <div
    class="relative bg-white px-12 py-2 pb-8"
    :class="{ 'font-mono': appearance.fontMono, 'text-sm': appearance.textSmall, 'text-md': !appearance.textSmall }"
  >
    <div class="mx-auto w-full max-w-[1000px]">
      <h2 class="mt-6 flex flex-row items-center gap-3">
        <button
          class="text-3xl font-bold text-gray-900 decoration-gray-900 underline-offset-4 hover:cursor-pointer hover:underline"
          @click="openRunsEditor"
        >
          Runs
        </button>
        <span class="mt-2 rounded-3xl bg-gray-100 py-0.5 px-2 text-sm text-gray-900" v-if="runsTableRef">
          {{ humanizeNumber(runsTableRef?.totalCount) }}
        </span>
      </h2>
      <RunsTable
        class="mt-2"
        ref="runsTableRef"
        :project-id="editor.currentProjectId"
        :project-version-id="editor.currentProjectVersionId"
        :include-ancestor-versions="true"
        override-from-props
      />
    </div>
  </div>
</template>

<script lang="ts" setup>
import ActionPopover from "@/components/basic/ActionPopover.vue";
import ClientsPopover from "@/components/basic/ClientsPopover.vue";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { useAppearance } from "@/state/appearance";
import { useAuth } from "@/state/auth";
import {
  FileEditor,
  StatementEditor,
  useBenchState,
  useEditorContext,
  type Action,
  type EditorType,
} from "@/state/bench";
import {
  ArrowsPointingInIcon,
  ArrowsPointingOutIcon,
  ChevronRightIcon,
  CodeBracketIcon,
  RocketLaunchIcon,
} from "@heroicons/vue/24/outline";
import { computed } from "vue";

const props = defineProps<{
  thing: any;
  actions: Action<any>[];
  path?: string | null;
  subpath?: string | null;
  editing: boolean;
}>();

const bench = useBenchState();
const editor = useEditorContext();
const editorAppearance = computed(() => editor.editor.value.appearance);
const appearance = useAppearance();
const auth = useAuth();

const editorIcons: Record<EditorType, any> = {
  file: CodeBracketIcon,
  statement: CodeBracketIcon,
  launch: RocketLaunchIcon,
};
const icon = computed(() => editorIcons[editor.editor.value.type]);
</script>
<template>
  <div
    class="fixed z-10 flex flex-row items-center justify-between gap-1 bg-white px-1.5"
    :class="appearance.baseClass"
    :style="{ height: appearance.editorHeaderHeight + 'px', width: editor.size?.value?.width + 'px' }"
  >
    <!-- Main info -->
    <div class="flex flex-row items-center">
      <!-- editor actions -->
      <ActionPopover anchor="left" :thing="thing" :actions="editor.actions.value" :groups="editor.actionGroups?.value">
        <component :is="icon" class="-mb-[3px] h-4 w-4 text-gray-700" />
      </ActionPopover>
      <!-- editor path -->
      <ActionPopover anchor="left" :thing="thing" :actions="actions" class="ml-1">
        <span class="text-gray-900">{{ path }}</span>
      </ActionPopover>
      <!-- sub path inside editor -->
      <FadeTransition mode="out-in">
        <span v-if="subpath" :key="subpath" class="flex flex-row text-gray-900"
          ><span class="text-gray-700"><ChevronRightIcon class="mr-0.5 mt-0.5 h-4 w-4 text-gray-400" /></span>
          {{ subpath }}</span
        >
      </FadeTransition>
      <span v-if="bench.debug" class="ml-2 bg-red-200 bg-opacity-50 text-gray-900">
        {{ editing ? "(editing)" : "" }}
        {{ bench.focusedEditorId == editor.editor.value.id ? "(focused)" : "" }}
      </span>
    </div>
    <div class="flex flex-row gap-1">
      <!-- Other clients presence -->
      <ClientsPopover
        v-if="auth.loggedIn.value && ['statement', 'file'].includes(editor.editor.value.type)"
        size="medium"
        :file-id="(editor.editor.value as FileEditor).fileId"
        :statement-id="(editor.editor.value as StatementEditor).statementId"
      />
      <!-- Extra inline actions -->
      <button
        class="rounded-sm p-0.5 text-gray-600 hover:bg-orange-100"
        @click.stop="editorAppearance.wide = !editorAppearance.wide"
      >
        <component :is="!editorAppearance.wide ? ArrowsPointingInIcon : ArrowsPointingOutIcon" class="h-4 w-4" />
      </button>
    </div>
  </div>
</template>

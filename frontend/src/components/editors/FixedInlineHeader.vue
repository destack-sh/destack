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
  usePanelContext,
  type Action,
  type PanelType,
} from "@/state/bench";
import {
  ArrowsPointingInIcon,
  ArrowsPointingOutIcon,
  ChevronRightIcon,
  CodeBracketIcon,
  WindowIcon,
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
const panel = usePanelContext();
const panelAppearance = computed(() => panel.panel.value.appearance);
const appearance = useAppearance();
const auth = useAuth();

// :EditorIcons
const editorIcons: Record<PanelType, any> = {
  file: CodeBracketIcon,
  statement: CodeBracketIcon,
  launch: WindowIcon,
};
const icon = computed(() => editorIcons[panel.panel.value.type]);
</script>
<template>
  <div
    class="group fixed z-10 flex flex-row items-center justify-between gap-1 bg-white px-1.5"
    :class="appearance.baseClass"
    :style="{ height: appearance.editorHeaderHeight + 'px', width: panel.size?.value?.width + 'px' }"
  >
    <!-- Main info -->
    <div class="flex flex-row items-center">
      <!-- editor actions -->
      <ActionPopover anchor="left" :thing="thing" :actions="panel.actions.value" :groups="panel.actionGroups?.value">
        <div class="p-0.5">
          <component :is="icon" class="h-4 w-4 text-gray-700" />
        </div>
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
        {{ bench.focusedPanelId == panel.panel.value.id ? "(focused)" : "" }}
      </span>
    </div>
    <div class="flex flex-row items-center gap-1">
      <!-- Other clients presence -->
      <ClientsPopover
        v-if="auth.loggedIn.value && ['statement', 'file'].includes(panel.panel.value.type)"
        size="small"
        :file-id="(panel.panel.value as FileEditor).fileId"
        :statement-id="(panel.panel.value as StatementEditor).statementId"
      />
      <!-- Extra inline actions -->
      <button
        class="rounded-sm p-0.5 text-gray-600 transition-opacity duration-150 hover:bg-orange-100"
        :class="panelAppearance.wide == undefined ? 'opacity-0 group-hover:opacity-100' : ''"
        @click.stop="panelAppearance.wide = !panel.panel.value.effectiveWide"
      >
        <component
          :is="!panel.panel.value.effectiveWide ? ArrowsPointingInIcon : ArrowsPointingOutIcon"
          class="h-4 w-4"
        />
      </button>
    </div>
  </div>
</template>

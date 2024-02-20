<script lang="ts" setup>
import ActionPopover from "@/components/basic/ActionPopover.vue";
import ClientsPopover from "@/components/basic/ClientsPopover.vue";
import { useAppearance } from "@/state/appearance";
import { useAuth } from "@/state/auth";
import {
  EditFilePanel,
  useBenchState,
  usePanelContext,
  type Action,
  PANEL_ICONS_OUTLINE,
  NavigablePanel,
} from "@/state/bench";
import type { NodeBase } from "@/state/module";
import {
  ArrowsPointingInIcon,
  ArrowsPointingOutIcon,
  ChevronRightIcon,
  EllipsisHorizontalIcon,
} from "@heroicons/vue/24/solid";
import { computed } from "vue";

type NamedElement = { __typename: string; id?: string; name?: string };
const props = defineProps<{
  thing: any;
  actions: Action<any>[];
  path: Array<NamedElement | NodeBase>;
  self: number;
  editing: boolean;
  hideWideToggle?: boolean;
}>();
const emit = defineEmits<{ (e: "focus", v: NamedElement | NodeBase): void }>();

const bench = useBenchState();
const panel = usePanelContext();
const panelAppearance = computed(() => panel.panel.value.appearance);
const appearance = useAppearance();
const auth = useAuth();
</script>
<template>
  <div
    class="group fixed z-10 flex flex-row items-center justify-between gap-1 px-1.5 text-xs"
    :class="appearance.baseClass"
    :style="{ height: appearance.panelHeaderHeight - 4 + 'px', width: panel.size?.value?.width + 'px' }"
  >
    <!-- Main info / left side -->
    <div class="flex flex-row items-center">
      <!-- Panel actions -->
      <ActionPopover
        anchor="left"
        small
        :thing="thing"
        :actions="panel.actions.value"
        :groups="panel.actionGroups?.value"
      >
        <div class="pb-0.5 pr-0.5">
          <component :is="PANEL_ICONS_OUTLINE[panel.panel.value.type]" class="h-4 w-4 text-gray-500" />
        </div>
      </ActionPopover>
      <!-- Panel path -->
      <div class="flex max-w-full flex-row items-center truncate whitespace-nowrap">
        <template v-for="(node, i) in path" :key="i">
          <!-- Self node with actions -->
          <span v-if="i == self" class="select-none px-0.5 font-semibold text-gray-500">
            {{ node.name ?? "(Unnamed)" }}
          </span>
          <!-- Regular node -->
          <button
            v-else-if="(node.name ?? '').length > 0 || i != path.length"
            class="group/node relative select-none rounded-sm px-0.5 text-gray-500 hover:bg-orange-100"
            @click.stop="
              i >= self || node.id == null
                ? emit('focus', node)
                : bench.focusNode(node as NodeBase, panel?.panel.value.group)
            "
          >
            <span>{{ node.name ?? "(Unnamed)" }}</span>
            <!-- Tooltip -->
            <span
              class="pointer-events-none absolute left-0 top-6 z-30 w-fit whitespace-nowrap rounded-sm bg-white px-1.5 text-xs text-gray-500 opacity-0 ring-1 ring-orange-900 ring-opacity-[25%] transition duration-150 group-hover/node:opacity-100"
            >
              {{
                i >= self
                  ? `Focus ${node.__typename.toLowerCase()} '${node.name}'`
                  : `Open ${node.__typename.toLowerCase()} '${node.name}'`
              }}
            </span>
          </button>
          <!-- Arrow -->
          <ChevronRightIcon v-if="i < path.length - 1" class="-mx-0.5 h-4 w-4 text-gray-400" />
        </template>
      </div>
      <!-- Debug info -->
      <span v-if="bench.debug" class="ml-2 bg-red-200 bg-opacity-50 text-gray-900">
        {{ editing ? "(editing)" : "" }}
        {{ bench.focusedPanelId == panel.panel.value.id ? "(focused)" : "" }}
        <template v-if="(panel.panel.value as NavigablePanel).activeStatementCk">
          (active:{{ (panel.panel.value as NavigablePanel).activeStatementCk }})
        </template>
      </span>
    </div>
    <!-- Right side secondary info / controls -->
    <div class="flex flex-row items-center gap-1">
      <!-- Other clients presence -->
      <ClientsPopover
        v-if="auth.loggedIn.value && ['statement', 'file'].includes(panel.panel.value.type)"
        size="small"
        :file-id="(panel.panel.value as EditFilePanel).fileCk"
      />
      <button
        v-if="!hideWideToggle"
        class="group/actoin relative rounded-sm p-0.5 text-gray-500 hover:bg-orange-100"
        @click.stop="panelAppearance.wide = !panel.panel.value.effectiveWide"
      >
        <component
          :is="!panel.panel.value.effectiveWide ? ArrowsPointingInIcon : ArrowsPointingOutIcon"
          class="h-4 w-4"
        />
      </button>
      <!-- Popover -->
      <ActionPopover v-if="actions.length > 0" anchor="left" small :thing="thing" :actions="actions" :groups="[]">
        <EllipsisHorizontalIcon class="h-4 w-4 text-gray-500" />
      </ActionPopover>
    </div>
  </div>
</template>

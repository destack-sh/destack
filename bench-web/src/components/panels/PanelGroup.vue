<script lang="ts" setup>
import { useElementRefs } from "@/composables/useGrid";
import PanelInterface from "@/components/panels/PanelInterface.vue";
import BlankPanel from "@/components/panels/BlankPanel.vue";
import { useActions } from "@/state/actions";
import { useAppearance } from "@/state/appearance";
import {
  useBenchState,
  type Panel,
  type PanelGroup,
  PANEL_ICONS_SOLID,
  getPanelActions,
  getPanelGroupActions,
  type FileHeader,
  type PanelType,
} from "@/state/bench";
import { Tab, TabGroup, TabList, TabPanel, TabPanels } from "@headlessui/vue";
import { PlusIcon, XMarkIcon } from "@heroicons/vue/24/outline";
import { useElementSize } from "@vueuse/core";
import { computed, nextTick, ref, watch, type Ref, watchEffect } from "vue";
import { useActiveScroll } from "@/composables/useScroll";
import { EllipsisVerticalIcon } from "@heroicons/vue/24/solid";
import { newNodeIdentity, useCurrentModule, type NodeBase } from "@/state/module";
import { useOperations } from "@/state/operations";
import { useNow } from "@/composables/useNow";
import { DateTime } from "luxon";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";

const props = defineProps<{ group: PanelGroup }>();

const bench = useBenchState();
const module = useCurrentModule();
const appearance = useAppearance();
const selectedTab = ref(-1);
const containerRef: Ref<HTMLDivElement | null> = ref(null);
const containerSize = useElementSize(containerRef);
// keep panel refs to pass to editor interface for scroll context
const panelRefs = useElementRefs<InstanceType<typeof TabPanel>>();
const tabListRef: Ref<InstanceType<typeof TabList> | null> = ref(null);
const tabRefs = useElementRefs<InstanceType<typeof Tab>>(computed(() => props.group.panels));

const KEEP_LOADED_RECENT_PANELS = 3;
const KEEP_LOADED_PANEL_TYPES: PanelType[] = ["launch-run", "terminal", "view-logs", "view-run", "view-runs"];
const KEEP_LOADED_COOLOFF_MS = 1000 * 30; // 30 seconds
const WAIT_UNTIL_MOUNTED_FOR_MS = 1000 * 6; // 6 seconds
const created = DateTime.now();

const now = useNow(1000);
const recentPanels = computed(() =>
  props.group.panels
    .slice()
    .sort((a, b) => (b.lastFocusedAt ?? "").localeCompare(a.lastFocusedAt ?? ""))
    .slice(0, KEEP_LOADED_RECENT_PANELS)
);
// wait to ensure active panel is loaded before loading the others
const stickyMountActive = computed(() => now.value.diff(created).milliseconds > WAIT_UNTIL_MOUNTED_FOR_MS);
function shouldKeepLoaded(panel: Panel): boolean {
  return (
    recentPanels.value.some((p) => p.id == panel.id) ||
    KEEP_LOADED_PANEL_TYPES.includes(panel.type) ||
    (panel.lastFocusedAt != null &&
      now.value.diff(DateTime.fromISO(panel.lastFocusedAt)).milliseconds < KEEP_LOADED_COOLOFF_MS)
  );
}

// context menu

const contextMenuPanel: Ref<Panel | null> = ref(null);
const contextMenuOpen = ref(false);
const contextMenuPosition: Ref<{ x: number; y: number } | null> = ref(null);
const contextMenuAnchor = ref("left" as "right" | "left");
const contextMenuActions = computed(() => {
  if (!contextMenuOpen.value) return [];
  return contextMenuPanel.value != null
    ? getPanelActions(contextMenuPanel.value, bench)
    : getPanelGroupActions(props.group, bench);
});

function closeContextMenu() {
  contextMenuOpen.value = false;
  contextMenuPanel.value = null;
  contextMenuPosition.value = null;
}

useActiveScroll(computed(() => tabListRef.value?.$el));

// auto update selected tab
watch(
  () => [props.group.activePanelId, props.group.panels],
  () => {
    if (props.group.activePanelId != null && props.group.panels.length > 0) {
      const activeEditorIndex = props.group.panels.findIndex((editor) => editor.id === props.group.activePanelId);
      if (activeEditorIndex < 0) {
        console.error(`active editor ${props.group.activePanelId} not found in group ${props.group.id}`);
      }
      if (selectedTab.value != activeEditorIndex) {
        // this used to work in the same tick, but headlessui now freaks
        // and lets its internal selectedIndex come out of sync with our controlled selectedTab
        // if we set it immediately. so we wait. 1 extra frame of latency..
        nextTick(() => (selectedTab.value = activeEditorIndex));
      }
    }
  },
  { immediate: true, deep: true }
);

// auto focus selected tab in horizontal scrolling
watch(selectedTab, () => {
  const tab = tabRefs.getRef(props.group.panels[selectedTab.value].id);
  // this is wrong if the tab is the last in an overflowing row
  //  (because our group actions popover is stickied floating to the right, overlapping with this)
  tab.$el?.scrollIntoView();
});

// compute editor size absolutely
const panelSize = computed(() => {
  return {
    width: containerSize.width.value + "px",
    height: containerSize.height.value - (bench.showPanelTabs ? appearance.panelHeaderHeight : 0) + "px",
  };
});

function focus(e: Panel) {
  bench.focusPanel(e);
  // blur any focused element when clicking on a tab
  bench.blur();
}

const ops = useOperations();
const actions = useActions();
async function createFileInPanelGroup() {
  const identity = newNodeIdentity(bench.projectVersionId as string, "File");
  const create = ops.file.create(null, identity.id, identity.ck, bench.projectVersionId as string, "", null);
  const optimisticFile = { __typename: "File", id: identity.id, ck: identity.ck, name };
  const panel = bench.focusFile(optimisticFile as NodeBase, props.group);
}
</script>
<template>
  <!-- Tabbed panels for this group -->
  <div class="relative flex flex-col" ref="containerRef">
    <!-- Tabs -->
    <TabGroup :selected-index="selectedTab" :default-index="selectedTab">
      <!-- Note that we use @click.prevent on the button instead of @onchange from TabGroup
       because we want to trigger re-focus even if it's already selected
      (happens if there are multiple active editor groups)  -->
      <!-- mr-8 to accommodate right-hand side context menu popup -->
      <TabList
        ref="tabListRef"
        class="scroll-hidden mr-8 flex w-full max-w-full flex-shrink-0 divide-x divide-orange-900 divide-opacity-[12%] overflow-x-scroll border-b border-orange-900/[12%] bg-gray-50"
        v-show="bench.showPanelTabs"
      >
        <!-- Editor tab -->
        <Tab
          v-for="(p, i) in group.panels"
          as="template"
          :key="p.id"
          :ref="(ref: any) => tabRefs.registerRef(p.id, ref)"
        >
          <button
            class="group relative flex max-w-[20rem] flex-shrink-0 select-none flex-row items-center gap-0.5 truncate text-ellipsis whitespace-nowrap py-[5px] pl-2 pr-1 outline-none transition duration-150"
            :class="{
              'bg-white text-gray-500 hover:bg-orange-100': i != selectedTab,
              'bg-orange-100 text-orange-600': i == selectedTab,
            }"
            @click.middle.stop.prevent="bench.closePanel(p)"
            @click.left.stop.prevent="focus(p)"
            @contextmenu.prevent="
              (e) => {
                contextMenuOpen = true;
                contextMenuPanel = p;
                contextMenuPosition = { x: e.clientX, y: e.clientY };
                contextMenuAnchor = 'left';
              }
            "
          >
            <component :is="PANEL_ICONS_SOLID[p.type]" class="mr-0.5 h-4 w-4 flex-shrink-0" />
            <span class="text-xs">{{ p.name.length > 0 ? p.name : "(Unnamed)" }}</span>
            <!-- Close button -->
            <button
              class="h-fit max-h-fit rounded-sm p-0.5 text-xs transition duration-150 hover:bg-gray-200 hover:text-gray-800"
              :class="i == selectedTab ? 'text-gray-400' : 'opacity-0 group-hover:opacity-100'"
              @click.stop.prevent="bench.closePanel(p)"
            >
              <XMarkIcon class="h-3 w-3" />
            </button>
            <!-- 'border' on bottom if tab is focused and active -->
            <div v-if="bench.focusedPanelId == p.id" class="absolute bottom-0 left-0 right-0 h-0.5 bg-orange-600" />
            <!-- 'border' on top if loaded for debugging -->
            <div v-if="bench.debug && shouldKeepLoaded(p)" class="absolute left-0 right-0 top-0 h-0.5 bg-green-600" />
          </button>
        </Tab>
        <!-- Little button tab to create new file -->
        <button
          v-if="!bench.readonly"
          @click="createFileInPanelGroup"
          class="group px-2 py-1 outline-none ring-0 hover:bg-red-100"
        >
          <PlusIcon class="h-4 w-4 text-gray-400 group-hover:text-gray-700" aria-hidden="true" />
        </button>
        <!-- Right-hand side context menu for group -->
        <button
          class="group absolute right-0 z-10 bg-gray-50 px-2 py-1 pb-1.5 outline-none ring-0 hover:bg-orange-100"
          @click.prevent="
            (e) => {
              contextMenuOpen = true;
              contextMenuPosition = { x: e.clientX, y: e.clientY };
              contextMenuAnchor = 'right';
            }
          "
        >
          <EllipsisVerticalIcon class="h-4 w-4 text-gray-400 group-hover:text-gray-700" aria-hidden="true" />
        </button>
      </TabList>
      <!-- Tab context menu -->
      <div v-if="contextMenuOpen && contextMenuPosition != null">
        <!-- Invisible fixed overlay to prevent scrolling and capture clicks -->
        <div class="fixed left-0 top-0 z-40 h-full w-full overscroll-none" @click.stop="closeContextMenu" />
        <!-- Tab context menu popover (similar to action popover) -->
        <div
          class="fixed z-50 flex w-40 flex-col rounded-sm bg-white p-1 text-xs shadow-md ring-1 ring-orange-900 ring-opacity-40"
          :style="{
            left: (contextMenuAnchor == 'left' ? contextMenuPosition.x : contextMenuPosition.x - 4 * 40) + 'px',
            top: contextMenuPosition.y + 'px',
          }"
        >
          <div
            v-for="(action, i) in contextMenuActions"
            :key="action.label"
            class="w-full"
            :class="[
              i > 0 && contextMenuActions[i - 1].groupId != action.groupId
                ? 'mt-0.5 border-t border-orange-900/[12%] pt-0.5'
                : '',
            ]"
            @click.prevent.stop="action.action((contextMenuPanel ?? group) as any), closeContextMenu()"
          >
            <button
              class="flex w-full flex-row items-center gap-2 rounded-sm px-1 py-1 hover:bg-orange-100 focus:outline-none"
              :class="[action.disabled || action.active ? 'cursor-not-allowed opacity-50' : '']"
            >
              <component :is="action.icon" class="h-4 w-4" />
              <span class="text-gray-700">{{ action.label }}</span>
            </button>
          </div>
        </div>
      </div>
      <!-- Contents -->
      <TabPanels :style="panelSize">
        <!-- Only source panels have a white background :PanelBackground -->
        <TabPanel
          v-for="p in group.panels"
          :key="p.id"
          :ref="(el: any) => panelRefs.registerRef(p.id, el)"
          as="div"
          class="outline-none"
          :class="[p.hasWhiteBackground ? 'bg-white' : 'bg-gray-50', p.hasScrollY ? 'overflow-y-scroll ' : '']"
          :style="panelSize"
          :unmount="!stickyMountActive || !shouldKeepLoaded(p)"
        >
          <PanelInterface :panel="p" :container-el="panelRefs.getRef(p.id)?.$el ?? null" />
        </TabPanel>
        <!-- Empty state -->
        <template v-if="bench.projectVersionId != null && group.activePanelId == null">
          <BlankPanel
            v-if="!module.loading.value"
            class="flex h-full w-full flex-col items-center justify-center"
            :group="group"
          />
          <div v-else class="relative flex h-full w-full flex-col justify-center">
            <BusySpinnerIcon class="mx-auto h-8 w-8 animate-spin text-gray-700" />
          </div>
        </template>
      </TabPanels>
    </TabGroup>
  </div>
</template>

<script lang="ts" setup>
import { useElementRefs } from "@/composables/useGrid";
import PanelInterface from "@/components/panels/PanelInterface.vue";
import BlankPanel from "@/components/panels/BlankPanel.vue";
import { useActions } from "@/state/actions";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type Panel, type PanelGroup, PANEL_ICONS_SOLID, getPanelActions } from "@/state/bench";
import { Tab, TabGroup, TabList, TabPanel, TabPanels } from "@headlessui/vue";
import { PlusIcon } from "@heroicons/vue/24/outline";
import { useElementSize } from "@vueuse/core";
import { computed, nextTick, ref, watch, type Ref } from "vue";
import { useActiveScroll } from "@/composables/useScroll";
const props = defineProps<{ group: PanelGroup }>();

const bench = useBenchState();
const appearance = useAppearance();
const selectedTab = ref(-1);
const containerRef: Ref<HTMLDivElement | null> = ref(null);
const containerSize = useElementSize(containerRef);
// keep panel refs to pass to editor interface for scroll context
const panelRefs = useElementRefs<InstanceType<typeof TabPanel>>();
const tabListRef: Ref<InstanceType<typeof TabList> | null> = ref(null);
const contextMenuPanel: Ref<Panel | null> = ref(null);
const contextMenuPosition: Ref<{ x: number; y: number } | null> = ref(null);
const contextMenuActions = computed(() =>
  contextMenuPanel.value == null ? [] : getPanelActions(contextMenuPanel.value, bench)
);

function closeContextMenu() {
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
      // this used to work in the same tick, but headlessui now freaks
      // and lets its internal selectedIndex come out of sync with our controlled selectedTab
      // if we set it immediately. so we wait. 1 extra frame of latency..
      nextTick(() => (selectedTab.value = activeEditorIndex));
    }
  },
  { immediate: true, deep: true }
);

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

const actions = useActions();
async function createFileInPanelGroup() {
  await actions.file.create.value.apply();
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
      <TabList
        ref="tabListRef"
        class="scroll-hidden flex w-full max-w-full flex-shrink-0 divide-x divide-orange-900 divide-opacity-[12%] overflow-x-scroll border-b border-orange-900 border-opacity-[12%] bg-gray-50"
        v-show="bench.showPanelTabs"
      >
        <!-- Editor tab -->
        <Tab as="template" v-for="(p, i) in group.panels" :key="p.id" v-slot="{ selected }">
          <button
            class="group relative flex max-w-[20rem] flex-shrink-0 select-none flex-row items-center gap-0.5 truncate text-ellipsis whitespace-nowrap py-[5px] pl-2 pr-1 outline-none"
            :class="{
              'bg-white text-gray-500 hover:text-orange-600': !selected,
              'bg-orange-100 text-orange-600': selected,
            }"
            @click.middle.prevent="bench.closePanel(p)"
            @click.left.prevent="focus(p)"
            @contextmenu.prevent="
              (e) => {
                contextMenuPanel = p;
                contextMenuPosition = { x: e.clientX, y: e.clientY };
              }
            "
          >
            <component :is="PANEL_ICONS_SOLID[p.type]" class="mr-0.5 h-4 w-4 flex-shrink-0" />
            <span class="text-xs">{{ p.name.length > 0 ? p.name : "(Unnamed)" }}</span>
            <!-- Close button -->
            <button
              class="h-fit max-h-fit rounded-sm px-1 text-xs transition duration-150 hover:bg-gray-200 hover:text-gray-700 group-hover:text-gray-400"
              :class="i == selectedTab ? 'text-gray-400' : 'opacity-0 group-hover:opacity-100'"
              @click.stop.prevent="bench.closePanel(p)"
            >
              x
            </button>
            <!-- 'border' on bottom if tab is focused and active -->
            <div v-if="bench.focusedPanelId == p.id" class="absolute bottom-0 left-0 right-0 h-0.5 bg-orange-600" />
          </button>
        </Tab>
        <!-- Little button tab to create new file -->
        <button
          @click="createFileInPanelGroup"
          class="group px-2 py-1 outline-none ring-0"
          :class="[actions.file.create.value.enabled ? 'hover:bg-orange-100' : 'opacity-30']"
          :disabled="!actions.file.create.value.enabled"
        >
          <PlusIcon class="h-4 w-4 text-gray-400 group-hover:text-gray-700" aria-hidden="true" />
        </button>
      </TabList>
      <!-- Tab context menu -->
      <div v-if="contextMenuPanel != null && contextMenuPosition != null">
        <!-- Invisible fixed overlay to prevent scrolling and capture clicks -->
        <div class="fixed left-0 top-0 z-40 h-full w-full overscroll-none" @click.stop="closeContextMenu" />
        <!-- Tab context menu popover (similar to action popover) -->
        <div
          class="fixed z-50 flex w-40 flex-col rounded-sm bg-white p-1 text-xs shadow-md ring-1 ring-orange-900 ring-opacity-40"
          :style="{ left: contextMenuPosition.x + 'px', top: contextMenuPosition.y + 'px' }"
        >
          <div
            v-for="(action, i) in contextMenuActions"
            :key="action.label"
            class="w-full"
            :class="[
              i > 0 && contextMenuActions[i - 1].groupId != action.groupId
                ? 'mt-0.5 border-t border-orange-900 border-opacity-[12%] pt-0.5'
                : '',
            ]"
            @click.prevent.stop="action.action(contextMenuPanel), closeContextMenu()"
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
        <!-- Only file panels have a white background :FileBackground -->
        <TabPanel
          :ref="(el: any) => panelRefs.registerRef(e.id, el)"
          as="div"
          class="overflow-y-scroll outline-none"
          :class="[e.hasWhiteBackground ? 'bg-white' : 'bg-gray-50']"
          :style="panelSize"
          v-for="e in group.panels"
          :key="e.id"
          unmount
        >
          <PanelInterface :panel="e" :container-el="panelRefs.getRef(e.id)?.$el ?? null" />
        </TabPanel>
        <BlankPanel
          v-if="bench.projectVersionId != null && group.activePanelId == null"
          class="relative h-full w-full"
          :group="group"
        />
      </TabPanels>
    </TabGroup>
  </div>
</template>

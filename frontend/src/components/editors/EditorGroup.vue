<script lang="ts" setup>
import { useElementRefs } from "@/composables/useGrid";
import EditorInterface from "@/components/editors/Editor.vue";
import EmptyEditor from "@/components/editors/EmptyEditor.vue";
import { useActions } from "@/state/actions";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type Editor, type EditorGroup } from "@/state/bench";
import { Tab, TabGroup, TabList, TabPanel, TabPanels } from "@headlessui/vue";
import { PlusIcon } from "@heroicons/vue/24/outline";
import { useElementSize } from "@vueuse/core";
import { computed, nextTick, ref, watch, type Ref } from "vue";
const props = defineProps<{ group: EditorGroup }>();

const bench = useBenchState();
const appearance = useAppearance();
const selectedTab = ref(-1);
const containerRef: Ref<HTMLDivElement | null> = ref(null);
const containerSize = useElementSize(containerRef);
// keep panel refs to pass to editor interface for scroll context
const panelRefs = useElementRefs<InstanceType<typeof TabPanel>>();
const focused = computed(() => bench.focusedEditor?.groupId == props.group.id);

// auto update selected tab
watch(
  () => [props.group.activeEditorId, props.group.editors],
  () => {
    if (props.group.activeEditorId != null && props.group.editors.length > 0) {
      const activeEditorIndex = props.group.editors.findIndex((editor) => editor.id === props.group.activeEditorId);
      if (activeEditorIndex < 0) {
        console.error(`active editor ${props.group.activeEditorId} not found in group ${props.group.id}`);
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
const editorSize = computed(() => {
  return {
    width: containerSize.width.value + "px",
    height: containerSize.height.value - (bench.showEditorGroupHeader ? appearance.editorHeaderHeight : 0) + "px",
  };
});

function focus(e: Editor) {
  bench.focusEditor(e);
  // blur any focused element when clicking on a tab
  bench.blur();
}

const actions = useActions();
async function createFileInEditorGroup() {
  await actions.file.create.value.apply();
}
</script>
<template>
  <!-- Tabbed editors for this group -->
  <div class="relative flex flex-col" ref="containerRef">
    <TabGroup :selected-index="selectedTab" :default-index="selectedTab">
      <!-- Tabs -->
      <!-- Note that we use @click.prevent on the button instead of @onchange from TabGroup
       because we want to trigger re-focus even if it's already selected
      (happens if there are multiple active editor groups)  -->
      <TabList
        class="scroll-hidden flex w-full max-w-full flex-shrink-0 overflow-x-scroll border-b border-orange-900 border-opacity-[12%] bg-gray-50"
        v-show="bench.showEditorGroupHeader"
        :style="{
          height: appearance.editorHeaderHeight + 'px',
        }"
      >
        <!-- Editor tab -->
        <Tab as="template" v-for="(e, i) in group.editors" :key="e.id" v-slot="{ selected }">
          <button
            class="group flex max-w-[20rem] flex-row items-center gap-0.5 truncate text-ellipsis whitespace-nowrap border-b-2 border-r border-r-gray-200 py-1 pl-3 pr-1 text-sm outline-none"
            :class="{
              'border-gray-50 bg-gray-50 text-gray-500 hover:text-orange-600': !selected,
              'bg-orange-100 text-orange-600': selected,
              'border-b-orange-600 ': selected && focused,
            }"
            @click.middle.prevent="bench.closeEditor(e)"
            @click.prevent="focus(e)"
          >
            {{ e.path.length > 0 ? e.path : "(Untitled)" }}
            <!-- Close button -->
            <button
              class="h-fit max-h-fit rounded-sm px-1 text-xs hover:bg-gray-200 group-hover:text-gray-700"
              :class="i == selectedTab ? 'text-gray-400' : 'text-transparent'"
              @click.prevent="bench.closeEditor(e)"
            >
              x
            </button>
          </button>
        </Tab>
        <!-- Little button tab to create new file -->
        <button
          v-if="actions.file.create.value.enabled"
          class="group mx-0.5 px-2 py-1 outline-none ring-0"
          @click="createFileInEditorGroup"
        >
          <PlusIcon
            class="h-4 w-4 text-gray-400 group-hover:bg-orange-100 group-hover:text-gray-700"
            aria-hidden="true"
          />
        </button>
      </TabList>
      <!-- Contents -->
      <TabPanels :style="editorSize">
        <!-- Only file editors have a white background :FileBackground -->
        <TabPanel
          :ref="(el: any) => panelRefs.registerRef(e.id, el)"
          as="div"
          class="overflow-y-scroll outline-none"
          :class="[e.hasWhiteBackground ? 'bg-white' : 'bg-gray-50']"
          :style="editorSize"
          v-for="e in group.editors"
          :key="e.id"
          unmount
        >
          <EditorInterface :editor="e" :container-el="panelRefs.getRef(e.id)?.$el ?? null" />
        </TabPanel>
        <EmptyEditor
          v-if="bench.currentProjectVersionId != null && group.activeEditorId == null"
          class="relative h-full w-full"
          :group="group"
        />
      </TabPanels>
    </TabGroup>
  </div>
</template>

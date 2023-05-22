<script lang="ts" setup>
import { useElementRefs } from "@/components/cells/grid";
import EditorInterface from "@/components/EditorInterface.vue";
import EmptyEditorInterface from "@/components/EmptyEditorInterface.vue";
import { useActions } from "@/state/actions";
import { useAppearance } from "@/state/appearance";
import { useEditorState, type Editor, type EditorGroup } from "@/state/editor";
import { Tab, TabGroup, TabList, TabPanel, TabPanels } from "@headlessui/vue";
import { PlusIcon } from "@heroicons/vue/24/outline";
import { useElementSize } from "@vueuse/core";
import { computed, nextTick, ref, toRef, watch, type Ref } from "vue";

const props = defineProps<{ group: EditorGroup }>();

const editor = useEditorState();
const appearance = useAppearance();
const selectedTab = ref(-1);
const containerRef: Ref<HTMLDivElement | null> = ref(null);
const containerSize = useElementSize(containerRef);
const mountAllPanels = ref(false);
// keep panel refs to pass to editor interface for scroll context
const panelRefs = useElementRefs<InstanceType<typeof TabPanel>>();
const focused = computed(() => editor.focusedEditor?.groupId == props.group.id);

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

const editorSize = computed(() => {
  return {
    width: containerSize.width.value + "px",
    height: containerSize.height.value - (editor.showEditorGroupHeader ? appearance.headerHeight : 0) + "px",
  };
});

function focus(e: Editor) {
  editor.focusEditor(e);
  // blur any focused element when clicking on a tab
  editor.blurElement();
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
      <!-- TODO @Robustness: prevent TabList from getting 'stuck' when scrolling down in content fast (that's what the sticky hack below 'solves') -->
      <TabList
        class="scroll-hidden flex w-full max-w-full flex-shrink-0 overflow-x-scroll border-b border-orange-900 border-opacity-[12%] bg-gray-50"
        v-show="editor.showEditorGroupHeader"
        :style="{
          height: appearance.headerHeight + 'px',
        }"
      >
        <Tab as="template" v-for="(e, i) in group.editors" :key="e.id" v-slot="{ selected }">
          <button
            class="group flex max-w-[20rem] flex-row items-center gap-0.5 truncate text-ellipsis whitespace-nowrap border-b-2 border-r border-r-gray-200 py-1 pl-3 pr-1 text-sm outline-none"
            :class="{
              'border-gray-50 bg-gray-50 text-gray-500 hover:text-orange-600': !selected,
              'bg-orange-100 text-orange-600': selected,
              'border-b-orange-600 ': selected && focused,
            }"
            @click.middle.prevent="editor.closeEditor(e)"
            @click.prevent="focus(e)"
          >
            {{ e.path.length > 0 ? e.path : "(Untitled)" }}
            <!-- Close button -->
            <button
              class="h-fit max-h-fit rounded-sm px-1 text-xs hover:bg-gray-200 group-hover:text-gray-700"
              :class="i == selectedTab ? 'text-gray-400' : 'text-transparent'"
              @click.prevent="editor.closeEditor(e)"
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
        <TabPanel
          :ref="(el: any) => panelRefs.registerRef(e.id, el)"
          as="div"
          class="overflow-y-scroll bg-white outline-none"
          :style="editorSize"
          v-for="e in group.editors"
          :key="e.id"
          :unmount="!mountAllPanels"
        >
          <EditorInterface :editor="e" :container-el="panelRefs.getRef(e.id)?.$el ?? null" />
        </TabPanel>
        <EmptyEditorInterface
          v-if="editor.currentProjectVersionId != null && group.editors.length === 0"
          class="relative h-full w-full"
          :group="group"
        />
      </TabPanels>
    </TabGroup>
  </div>
</template>

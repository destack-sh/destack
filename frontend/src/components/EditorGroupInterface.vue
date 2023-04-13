<script lang="ts" setup>
import EditorInterface from "@/components/EditorInterface.vue";
import EmptyEditorInterface from "@/components/EmptyEditorInterface.vue";
import { useActions } from "@/state/actions";
import { useEditorState, type Editor, type EditorGroup } from "@/state/editor";
import { Tab, TabGroup, TabList, TabPanel, TabPanels } from "@headlessui/vue";
import { PlusIcon } from "@heroicons/vue/24/outline";
import { computed, nextTick, ref, toRef, watch } from "vue";

const props = defineProps<{ group: EditorGroup }>();

const editor = useEditorState();
const selectedTab = ref(0);
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
const focused = computed(() => editor.focusedEditor?.groupId == props.group.id);

function focus(e: Editor) {
  editor.focusEditor(e);
  // blur any focused element when clicking on a tab
  editor.blurElement();
}

// load panels (i.e. disallow unmounting) after 2s to load active panel first
const mountAllPanels = ref(false);
setTimeout(() => {
  mountAllPanels.value = true;
}, 2000);
// reset whenever project version changes
watch(
  toRef(editor, "currentProjectVersionId"),
  () => (
    (mountAllPanels.value = false),
    setTimeout(() => {
      mountAllPanels.value = true;
    }, 2000)
  )
);

const actions = useActions();
async function createFileInEditorGroup() {
  await actions.file.create.value.apply();
}
</script>
<template>
  <!-- Tabbed editors for this group -->
  <div class="relative flex flex-col">
    <TabGroup :selected-index="selectedTab">
      <!-- Tabs -->
      <!-- Note that we use @click.prevent on the button instead of @onchange from TabGroup
       because we want to trigger re-focus even if it's already selected
      (happens if there are multiple active editor groups)  -->
      <!-- TODO @Robustness: prevent TabList from getting 'stuck' when scrolling down in content fast -->
      <TabList
        class="sticky top-0 z-[5] flex flex-shrink-0 border-b border-orange-900 border-opacity-[12%] bg-gray-50"
        v-show="editor.showEditorGroupHeader"
      >
        <Tab as="template" v-for="e in group.editors" :key="e.id" v-slot="{ selected }">
          <button
            :class="{
              'max-w-[20rem] truncate text-ellipsis whitespace-nowrap border-b-2 border-r border-r-gray-200 px-3 py-1 text-sm outline-none': true,
              'border-gray-50 bg-gray-50 text-gray-500 hover:text-orange-600': !selected,
              'bg-orange-100 text-orange-600': selected,
              'border-b-orange-600 ': selected && focused,
            }"
            @click.middle.prevent="editor.closeEditor(e)"
            @click.prevent="focus(e)"
          >
            {{ e.path.length > 0 ? e.path : "(Untitled)" }}
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
      <TabPanels class="relative h-full w-full flex-1">
        <!-- TODO @Robustness: handle resizable scrollable flex containers (editor, views) better -->
        <!-- Scrolling currently relies on this weird relative/absolute hack, but it's not
             easy to apply to proper resizable elements and it cuts off areas (e.g. the bottom),
            because it includes more width/height than it should, so we have extra padding
            (e.g. in FileInterface and ViewExplorer/ViewHistory/etc.)
            -->
        <TabPanel
          as="div"
          class="h-full w-full overflow-y-scroll bg-white outline-none"
          v-for="e in group.editors"
          :key="e.id"
          :unmount="!mountAllPanels"
        >
          <EditorInterface :editor="e" />
        </TabPanel>
        <EmptyEditorInterface
          v-if="editor.currentProjectVersionId != null && group.editors.length === 0"
          class="relative h-full w-full flex-1"
        />
      </TabPanels>
    </TabGroup>
  </div>
</template>

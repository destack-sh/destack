<script lang="ts" setup>
import EditorInterface from "@/components/EditorInterface.vue";
import { useActions } from "@/state/actions";
import { useEditorState, type Editor, type EditorGroup } from "@/state/editor";
import { Tab, TabGroup, TabList, TabPanel, TabPanels } from "@headlessui/vue";
import { PlusIcon } from "@heroicons/vue/24/outline";
import { computed, ref, watch } from "vue";

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
      selectedTab.value = activeEditorIndex;
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

const actions = useActions();
async function createFileInEditorGroup() {
  await actions.file.create.value.apply();
}
</script>
<template>
  <!-- Tabbed editors for this group -->
  <div class="flex flex-col">
    <TabGroup :selected-index="selectedTab">
      <!-- Tabs -->
      <!-- Note that we use @click.prevent on the button instead of @onchange from TabGroup
       because we want to trigger re-focus even if it's already selected
      (happens if there are multiple active editor groups)  -->
      <TabList
        class="flex flex-shrink-0 border-b border-orange-900 border-opacity-[12%]"
        v-show="editor.showEditorGroupHeader"
      >
        <Tab as="template" v-for="e in group.editors" :key="e.id" v-slot="{ selected }">
          <button
            :class="{
              'max-w-[20rem] truncate text-ellipsis whitespace-nowrap border-r border-b-2 border-r-gray-200 py-1 px-3 text-sm outline-none': true,
              'border-gray-50 bg-gray-50 text-gray-500 hover:text-orange-600': !selected,
              'bg-orange-100 text-orange-600': selected,
              'border-b-orange-600 ': selected && focused,
            }"
            @click.middle.prevent="editor.closeEditor(e)"
            @click.prevent="focus(e)"
          >
            {{ e.path }}
          </button>
        </Tab>
        <!-- Little button tab to create new file -->
        <button
          v-if="actions.file.create.value.enabled"
          class="group mx-0.5 py-1 px-2 outline-none ring-0"
          @click="createFileInEditorGroup"
        >
          <PlusIcon class="h-4 w-4 text-gray-400 group-hover:text-gray-500" aria-hidden="true" />
        </button>
      </TabList>
      <!-- Contents -->
      <TabPanels class="relative h-full w-full flex-1">
        <TabPanel
          as="div"
          class="h-full w-full overflow-auto bg-white outline-none"
          v-for="e in group.editors"
          :key="e.id"
          :unmount="!mountAllPanels"
        >
          <EditorInterface :editor="e" />
        </TabPanel>
      </TabPanels>
    </TabGroup>
  </div>
</template>

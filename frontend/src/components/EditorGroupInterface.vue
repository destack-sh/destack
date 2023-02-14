<script lang="ts" setup>
import EditorInterface from "@/components/EditorInterface.vue";
import { useActions } from "@/state/actions";
import { useEditorState, type Editor, type EditorGroup } from "@/state/editor";
import { Tab, TabGroup, TabList, TabPanel, TabPanels } from "@headlessui/vue";
import { PlusIcon } from "@heroicons/vue/24/outline";
import { computed, ref, watch } from "vue";

const props = defineProps<{ group: EditorGroup }>();

const editorState = useEditorState();
const selectedTab = ref(0);
watch(
  () => [props.group.activeEditor, props.group.editors],
  () => {
    if (props.group.activeEditor != null && props.group.editors.length > 0) {
      const activeEditorIndex = props.group.editors.findIndex((editor) => editor.id === props.group.activeEditor?.id);
      if (activeEditorIndex < 0) {
        console.error(`active editor ${props.group.activeEditor?.id} not found in group ${props.group.id}`);
      }
      selectedTab.value = activeEditorIndex;
    }
  }
);
const focused = computed(() => editorState.focusedEditor?.groupId == props.group.id);

function focus(editor: Editor) {
  editorState.focusEditor(editor);
  // blur any focused element when clicking on a tab
  editorState.blurElement();
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
      <TabList class="flex border-b border-gray-200">
        <Tab as="template" v-for="editor in group.editors" :key="editor.id" v-slot="{ selected }">
          <button
            :class="{
              'max-w-[20rem] truncate text-ellipsis whitespace-nowrap border-r border-b-2 border-r-gray-200 py-2 px-3 text-sm outline-none': true,
              'border-gray-50 bg-gray-50 text-gray-700 hover:text-orange-600': !selected,
              ' bg-orange-100 text-orange-600': selected,
              'border-b-orange-600 ': selected && focused,
            }"
            @click.middle="editorState.closeEditor(editor)"
            @click.prevent="focus(editor)"
          >
            {{ editor.path }}
          </button>
        </Tab>
        <!-- Little button tab to create new file -->
        <button class="group mx-0.5 p-1 outline-none ring-0" @click="createFileInEditorGroup">
          <PlusIcon class="h-4 w-4 text-gray-300 group-hover:text-gray-500" aria-hidden="true" />
        </button>
      </TabList>
      <!-- Contents -->
      <TabPanels class="relative h-full w-full flex-1">
        <TabPanel
          as="div"
          class="h-full w-full overflow-auto bg-white outline-none"
          v-for="editor in group.editors"
          :key="editor.id"
          :unmount="!mountAllPanels"
        >
          <EditorInterface :editor="editor" />
        </TabPanel>
      </TabPanels>
    </TabGroup>
  </div>
</template>

<script lang="ts" setup>
import FileInterface from "@/components/FileInterface.vue";
import { useEditorState, type EditorGroup, type FileEditor } from "@/utils/editor";
import { Tab, TabGroup, TabList, TabPanel, TabPanels } from "@headlessui/vue";
import { ref, watch } from "vue";

const props = defineProps<{ group: EditorGroup }>();

const editorState = useEditorState();
const selectedTab = ref(0);
watch(
  () => [props.group.activeEditor, props.group.editors],
  () => {
    console.log(`update selected tab in ${props.group.id}`, props.group);
    if (props.group.activeEditor != null && props.group.editors.length > 0) {
      const activeEditorIndex = props.group.editors.findIndex((editor) => editor.id === props.group.activeEditor?.id);
      if (activeEditorIndex < 0) {
        console.error(`active editor ${props.group.activeEditor?.id} not found in group ${props.group.id}`);
      }
      selectedTab.value = activeEditorIndex;
    }
  }
);

function changeTab(index: number) {
  editorState.focusEditor(props.group.editors[index]);
}
</script>
<template>
  <!-- Tabbed editors for that group -->
  <div>
    <TabGroup :selected-index="selectedTab" @change="changeTab">
      <!-- Tabs -->
      <TabList class="flex border-b border-gray-200">
        <Tab as="template" v-for="editor in group.editors" :key="editor.path" v-slot="{ selected }">
          <button
            :class="{
              'border-r border-b-2 border-r-gray-200 py-2 px-3 text-sm outline-none': true,
              'border-gray-50 bg-gray-50 text-gray-700 hover:text-orange-600': !selected,
              'border-b-orange-600 bg-orange-100 text-orange-600': selected,
            }"
          >
            {{ editor.path }}
          </button>
        </Tab>
      </TabList>
      <TabPanels class="w-full">
        <TabPanel v-for="(editor, index) in group.editors" :key="index">
          <div class="h-full w-full">
            <FileInterface v-if="editor.type == 'file'" :file="(editor as FileEditor).file" />
          </div>
        </TabPanel>
      </TabPanels>
    </TabGroup>
  </div>
</template>

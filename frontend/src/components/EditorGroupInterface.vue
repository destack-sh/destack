<script lang="ts" setup>
import EditorInterface from "@/components/EditorInterface.vue";
import FileInterface from "@/components/FileInterface.vue";
import RunInterface from "@/components/RunInterface.vue";
import { useEditorState, type Editor, type EditorGroup, type FileEditor, type RunEditor } from "@/utils/editor";
import { Tab, TabGroup, TabList, TabPanel, TabPanels } from "@headlessui/vue";
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

function shouldUnmountPanel(editor: Editor): boolean {
  // TODO @Performance: decide when to unmount panels in editor group
  return false;
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
        <Tab as="template" v-for="editor in group.editors" :key="editor.path" v-slot="{ selected }">
          <button
            :class="{
              'border-r border-b-2 border-r-gray-200 py-2 px-3 text-sm outline-none': true,
              'border-gray-50 bg-gray-50 text-gray-700 hover:text-orange-600': !selected,
              ' bg-orange-100 text-orange-600': selected,
              'border-b-orange-600 ': selected && focused,
            }"
            @click.middle="editorState.closeEditor(editor)"
            @click.prevent="editorState.focusEditor(editor)"
          >
            {{ editor.path }}
          </button>
        </Tab>
      </TabList>
      <!-- Contents -->
      <TabPanels class="relative h-full w-full flex-1">
        <TabPanel v-for="(editor, index) in group.editors" :key="index" :unmount="shouldUnmountPanel(editor)">
          <EditorInterface class="absolute left-0 top-0 h-full w-full overflow-auto" :editor="editor" />
        </TabPanel>
      </TabPanels>
    </TabGroup>
  </div>
</template>

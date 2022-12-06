<script lang="ts" setup>
import { EDITOR_STATE_KEY, type EditorGroup, type EditorState } from "@/utils/editor";
import { Tab, TabGroup, TabList, TabPanel, TabPanels } from "@headlessui/vue";
import { inject, ref } from "vue";

defineProps<{ group: EditorGroup }>();
const state = inject<EditorState>(EDITOR_STATE_KEY);

const selectedTab = ref(0);

function changeTab(index: number) {
  selectedTab.value = index;
}
</script>
<template>
  <!-- Tabbed editors for that group -->
  <div>
    <TabGroup :selected-index="selectedTab" @change="changeTab">
      <!-- Tabs -->
      <TabList class="flex divide-x divide-gray-200 border-b border-gray-200">
        <Tab as="template" v-for="editor in group.editors.value" :key="editor.path.value" v-slot="{ selected }">
          <button
            :class="{
              'rounded-sm border-b-2 p-2 text-sm': true,
              'border-gray-50 bg-gray-50 text-gray-700': !selected,
              'border-orange-600 bg-orange-100 text-orange-600': selected,
            }"
          >
            {{ editor.path }}
          </button>
        </Tab>
      </TabList>
      <TabPanels>
        <TabPanel v-for="editor in group.editors.value" :key="editor.path.value">
          {{ editor.path }}
          <!-- <div class="h-full w-full">
            <FileInterface v-if="editor.type == 'file'" :file="(editor as FileEditor).file" />
          </div> -->
        </TabPanel>
      </TabPanels>
    </TabGroup>
  </div>
</template>

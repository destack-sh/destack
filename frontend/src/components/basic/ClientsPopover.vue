<script lang="ts" setup>
import { getClientColor, useConnectedClients } from "@/state/client";
import { useEditorState } from "@/state/editor";
import { toRef, ref, computed } from "vue";

const editor = useEditorState();
const { totalCount, clientsWithoutSelf: clients } = useConnectedClients(
  {
    projectId: toRef(editor, "currentProjectId"),
    projectVersionId: toRef(editor, "currentProjectVersionId"),
    userId: ref(null),
    inSameOrganizations: computed(() => editor.currentProjectId == null),
  },
  {
    live: true,
    first: 4,
  }
);
</script>
<template>
  <div class="flex flex-row gap-1">
    <!-- :ProfilePreview -->
    <div
      v-for="client in clients"
      :key="client.id"
      class="group relative border border-orange-900 border-opacity-[15%] bg-orange-100 px-2.5 py-1"
      :style="{
        backgroundColor: getClientColor(client.id),
      }"
    >
      <span class="text-sm font-bold text-gray-900">
        {{ client.user.username.slice(0, 2).toLocaleUpperCase() }}
      </span>
      <!-- Info popover -->
      <div
        class="absolute right-0 z-10 mt-3 w-40 origin-bottom-right bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-orange-900 ring-opacity-40 group-hover:visible"
      >
        {{ client.user.username }}
        {{ client.user.email }}
        {{ client.browserName }} {{ client.deviceName }}
      </div>
    </div>
  </div>
</template>

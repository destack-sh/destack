<script lang="ts" setup>
import { getClientColor, useConnectedClients } from "@/state/client";
import { useEditorState } from "@/state/editor";
import { toRef, ref } from "vue";

const editor = useEditorState();
const { totalCount, clientsWithoutSelf: clients } = useConnectedClients(
  {
    projectId: toRef(editor, "currentProjectId"),
    projectVersionId: toRef(editor, "currentProjectVersionId"),
    userId: ref(null),
    inSameOrganizations: ref(true),
  },
  {
    live: true,
    first: 4,
  }
);
</script>
<template>
  <div class="flex flex-row">
    <div
      v-for="client in clients"
      :key="client.id"
      class="border border-orange-900 border-opacity-[15%] bg-orange-100 px-2 py-1"
      :style="{
        backgroundColor: getClientColor(client.id),
      }"
    >
      <!-- should be two proper letters or profile pic? -->
      {{ client.user.username.slice(0, 2).toLocaleUpperCase() }}
    </div>
  </div>
</template>

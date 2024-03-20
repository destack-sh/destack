<script lang="tsx" setup>
import { DECLARED_ACTIONS_BY_ID, type Action, type ActionBuiltinId, fireAction } from "@/system/action";
import { IconInline } from "@/system/icon";
import { Tooltip } from "@/utils/tooltip";
import { computed, type Ref } from "vue";

const actions: Ref<Action[]> = computed(
  () =>
    (
      [
        "space.launch.omnibar.actions",
        "space.launch.omnibar.space",
        "space.launch.inspector",
        "space.launch.library",
        "space.launch.docs",
        "space.launch.discord",
      ] as ActionBuiltinId[]
    )
      .map((id) => DECLARED_ACTIONS_BY_ID.value[id])
      .filter((a) => a != null) as Action[],
);
</script>
<template>
  <div class="flex flex-row items-center gap-x-2">
    <component
      :is="action.url ? 'a' : 'button'"
      v-for="action in actions"
      :key="action.id"
      class="group relative rounded-md border border-gray-900 bg-primary-300 px-1 py-0.5 text-gray-900 shadow-sm shadow-gray-900 hover:cursor-pointer hover:bg-primary-400"
      @click="fireAction(action)"
      :href="action.url"
      target="_blank"
    >
      <IconInline v-bind="action.icon" />
      <Tooltip
        :icon="action.icon"
        :title="action.title"
        :text="action.text as string"
        :shortcut="action.shortcuts?.[0]"
        position="top-7 -left-3"
      />
    </component>
  </div>
</template>

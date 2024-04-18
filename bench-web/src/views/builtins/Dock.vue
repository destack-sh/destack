<script lang="ts" setup>
import { DECLARED_ACTIONS_BY_ID, fireAction, type Action, type ActionBuiltinId } from "@/system/action";
import { tooltipFromAction } from "@/utils/tooltip";
import Button from "@/views/controls/Button.vue";
import { computed, ref, type Component, type Ref } from "vue";

const actions: Ref<Action[]> = computed(
  () =>
    (
      [
        "space.omnibar.actions",
        "space.omnibar.space",
        "space.edit.inspect",
        "space.launch.create",
        "space.launch.assist",
        "space.launch.docs",
      ] as ActionBuiltinId[]
    )
      .map((id) => DECLARED_ACTIONS_BY_ID.value[id])
      .filter((a) => a != null) as Action[],
);
const actionRefs: Ref<Record<string, Component<typeof Button>>> = ref({});
</script>
<template>
  <div class="flex flex-row items-center gap-x-2">
    <template v-for="action in actions" :key="action.id">
      <Button
        :ref="(ref?: any) => (ref != null ? (actionRefs[action.id] = ref) : (delete actionRefs[action.id]))"
        :icon="action.icon"
        :name="action.id"
        @click="fireAction(action, null)"
        v-tooltip="tooltipFromAction(action)"
      />
    </template>
  </div>
</template>
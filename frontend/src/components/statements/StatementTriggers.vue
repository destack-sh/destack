<script lang="ts" setup>
import type { Trigger } from "@/gql/graphql";
import { TriggerType } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { useCurrentModule } from "@/state/module";
import { useOperations } from "@/state/operations";
import { TRIGGER_ICONS_SOLID, useStatementContext } from "@/state/statement";
import { PlusIcon, BoltIcon } from "@heroicons/vue/24/outline";
import { ref } from "vue";

const context = useStatementContext();
const module = useCurrentModule();
const appearance = useAppearance();
const ops = useOperations();

const adding = ref(false);
const editingId = ref<string | null>(null);

function open() {
  adding.value = true;
}

function editTrigger(trigger: Trigger) {
  editingId.value = trigger.id;
}

function renderTrigger(trigger: Trigger): string {
  return "every 1h"; // nocheckin
}
</script>
<template>
  <div class="group relative flex flex-row gap-1.5">
    <!-- Existing triggers -->
    <span
      v-for="trigger in context.triggers.value"
      :key="trigger.id"
      class="flex flex-row items-center rounded-xl bg-orange-100 px-1.5 text-orange-900 ring-1 ring-inset ring-orange-600/20"
      @click="editTrigger(trigger)"
    >
      <component :is="TRIGGER_ICONS_SOLID[TriggerType.Time]" class="mr-1 h-4 w-4" />
      <span class="text-sm">every 1h</span>
    </span>
    <!-- Add trigger button -->
    <button
      v-if="!context.readonly.value"
      class="group/add flex flex-row rounded-xl border-gray-600 border-opacity-25 px-1 py-0 text-gray-400 hover:bg-orange-100 hover:text-gray-700 group-hover/add:ring-1"
      :class="
        context.focused.value
          ? ''
          : 'opacity-0 transition-opacity duration-150 group-hover/statement:opacity-100 group-hover:opacity-100'
      "
      @click="open"
    >
      <BoltIcon class="mt-0.5 h-4 w-4" />
      <!-- <PlusIcon class="ml-1 mt-0.5 h-4 w-4 opacity-0 transition-opacity duration-150 group-hover/add:opacity-100" /> -->
    </button>
    <!-- Adding - select trigger type -->
    <!-- Editing -->
  </div>
</template>

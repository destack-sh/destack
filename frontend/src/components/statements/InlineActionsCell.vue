<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { useMagicActions } from "@/state/file";
import type { StatementAction } from "@/state/bench";
import { useStatementContext } from "@/state/statement";
import type { StatementHeader } from "@/state/bench";
import { Square2StackIcon } from "@heroicons/vue/24/outline";
import { computed, type Ref } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";

const props = defineProps<{
  extraActions?: StatementAction[];
}>();

const context = useStatementContext();
const magic = useMagicActions(context.statement as Ref<StatementHeader>);

const inlineActions: Ref<StatementAction[]> = computed(() => {
  const inlineActions: StatementAction[] = [];
  if (props.extraActions) {
    inlineActions.push(...props.extraActions);
  }
  if (!context.readonly.value) {
    inlineActions.push({
      label: "Duplicate",
      icon: Square2StackIcon,
      action: () => magic.duplicate(),
    });
  }
  return inlineActions;
});
</script>
<template>
  <span class="flex flex-row items-center gap-1 p-0.5">
    <slot name="before" />
    <button
      v-for="action in inlineActions.filter((action) => !action.hideInline && !action.disabled)"
      :key="action.label"
      class="group relative p-0.5 text-gray-500 hover:text-gray-800"
      :class="action.active ? 'animate-spin cursor-not-allowed' : 'hover:bg-orange-100'"
      @click.prevent.stop="action.action(context.statement.value as StatementHeader)"
      :disabled="action.disabled || action.active"
    >
      <FadeTransition name="fade" mode="out-in">
        <component :is="action.active ? BusySpinnerIcon : action.icon" class="h-4 w-4" />
      </FadeTransition>
      <!-- Label -->
      <span
        v-if="!action.active"
        class="pointer-events-none absolute -left-3 top-6 z-10 whitespace-nowrap rounded-sm border border-orange-900 border-opacity-[15%] bg-white px-2 text-gray-900 opacity-0 transition duration-150 group-hover:opacity-100"
      >
        {{ action.label }}
      </span>
    </button>
    <slot name="after" />
  </span>
</template>

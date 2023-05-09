<script lang="ts" setup>
import { useMagicActions } from "@/components/file";
import { useStatementContext } from "@/components/statement";
import type { StatementHeader } from "@/state/editor";
import { DocumentDuplicateIcon, ArrowPathIcon } from "@heroicons/vue/24/outline";
import { computed, type Ref } from "vue";
import type { InlineAction } from "@/components/statement";

const props = defineProps<{
  extraActions?: InlineAction[];
}>();

const context = useStatementContext();
const magic = useMagicActions(context.statement as Ref<StatementHeader>);

const inlineActions: Ref<InlineAction[]> = computed(() => {
  const inlineActions: InlineAction[] = [];
  if (!context.readonly.value) {
    inlineActions.push({
      label: "Duplicate",
      icon: DocumentDuplicateIcon,
      action: () => magic.duplicate(),
    });
  }
  if (props.extraActions) {
    inlineActions.push(...props.extraActions);
  }
  return inlineActions;
});
</script>
<template>
  <span class="flex flex-row items-center gap-1 p-0.5">
    <slot name="before" />
    <button
      v-for="action in inlineActions"
      :key="action.label"
      class="p-0.5 text-gray-500 hover:text-gray-800"
      :class="action.active ? 'animate-spin cursor-not-allowed' : 'hover:bg-orange-100'"
      @click.prevent.stop="action.action"
      :disabled="action.disabled || action.active"
    >
      <component :is="action.active ? ArrowPathIcon : action.icon" class="h-4 w-4" />
    </button>
    <slot name="after" />
  </span>
</template>

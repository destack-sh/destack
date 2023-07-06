<script lang="ts" setup>
import StatementActions from "@/components/statements/StatementActions.vue";
import { useStatementOps } from "@/state/operations/statement";
import { getStatementIcon, useStatementContext } from "@/state/statement";
import { computed } from "vue";

const props = defineProps<{ folded?: boolean }>();
const emit = defineEmits<{ (e: "toggleFold"): void }>();

const context = useStatementContext();
const ops = useStatementOps();
const icon = computed(() => getStatementIcon(context.statement.value.type, context.statement.value.rootTypeTag));

function focus(position: "first" | "last" = "first") {
  // nocheckin
}

function blur() {
  // nocheckin
}

defineExpose({
  focus,
  blur,
});
</script>
<template>
  <div class="flex flex-row justify-between">
    <!-- Declaration -->
    <div class="flex flex-row items-baseline">
      <component :is="icon" class="h-4 w-4 text-orange-600" />
    </div>
    <!-- Controls -->
    <StatementActions />
  </div>
</template>

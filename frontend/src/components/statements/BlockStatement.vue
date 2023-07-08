<script lang="ts" setup>
import StatementActions from "@/components/statements/StatementActions.vue";
import StatementDeclaration from "@/components/statements/StatementDeclaration.vue";
import StatementTags from "@/components/statements/StatementTags.vue";
import { useStatementContext } from "@/state/statement";
import { ref } from "vue";

const props = defineProps<{ folded?: boolean }>();
const emit = defineEmits<{ (e: "toggleFold"): void }>();

const declarationRef = ref<InstanceType<typeof StatementDeclaration> | null>(null);
const tagsRef = ref<InstanceType<typeof StatementTags> | null>(null);
const context = useStatementContext();

function focus(position: "first" | "last" = "first") {
  declarationRef.value?.focus();
}

function blur() {
  declarationRef.value?.blur();
}

defineExpose({
  focus,
  blur,
});
</script>
<template>
  <div class="flex flex-row justify-between">
    <!-- Declaration -->
    <div class="flex flex-row">
      <StatementDeclaration ref="declarationRef" />
      <StatementTags ref="tagsRef" class="ml-1.5" />
    </div>
    <!-- Controls -->
    <div
      class="flex flex-row items-center gap-1 transition duration-150 group-hover/statement:opacity-100"
      :class="context.focused.value ? '' : 'opacity-0'"
    >
      <StatementActions />
    </div>
  </div>
  <!-- Description and stuff.. soon -->
</template>

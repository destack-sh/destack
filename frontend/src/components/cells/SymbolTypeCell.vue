<script lang="ts" setup>
import { useStatementContext } from "@/components/statement";
import { SYMBOL_TYPE_KEYWORD } from "@/state/editor";
import { computed } from "vue";

const context = useStatementContext();
const symbolTypeKeyword = computed(() => SYMBOL_TYPE_KEYWORD[context.statement.value.symbolType]);

defineExpose({
  delete: () => context.morphSetSymbolType(null),
});
</script>
<template>
  <span class="text-orange-600" :contenteditable="false">{{ symbolTypeKeyword }}</span>
</template>

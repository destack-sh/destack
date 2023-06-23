<script lang="ts" setup>
import { useStatementContext, getStatementIcon } from "@/state/statement";
import { STATEMENT_TYPE_KEYWORD } from "@/state/type";
import { computed } from "vue";

const context = useStatementContext();

const showIcon = true;
const icon = computed(() => getStatementIcon(context.statement.value.type, context.statement.value.rootTypeTag));
</script>
<template>
  <component v-if="showIcon" :is="icon" class="mt-0.5 h-4 w-4 text-gray-700" />
  <span v-else class="text-orange-600" :contenteditable="false">
    {{ context.symbolSubtype.value ?? STATEMENT_TYPE_KEYWORD[context.statement.value.type] }}
  </span>
</template>

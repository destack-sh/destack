<script lang="ts" setup>
import { useStatementContext } from "@/components/statement";
import { SYMBOL_TYPE_KEYWORD } from "@/state/editor";

const context = useStatementContext();
</script>
<template>
  <span class="text-orange-600" :contenteditable="false">
    {{ SYMBOL_TYPE_KEYWORD[context.statement.value.symbolType] }}
  </span>
</template>

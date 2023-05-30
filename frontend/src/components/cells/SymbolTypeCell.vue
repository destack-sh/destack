<script lang="ts" setup>
import { useStatementContext } from "@/state/statement";
import { SYMBOL_TYPE_KEYWORD } from "@/state/type";

const context = useStatementContext();
</script>
<template>
  <span class="text-orange-600" :contenteditable="false">
    {{ context.symbolSubtype.value ?? SYMBOL_TYPE_KEYWORD[context.statement.value.symbolType] }}
  </span>
</template>

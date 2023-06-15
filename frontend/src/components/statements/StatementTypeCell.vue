<script lang="ts" setup>
import { useStatementContext } from "@/state/statement";
import { STATEMENT_TYPE_KEYWORD } from "@/state/type";

const context = useStatementContext();
</script>
<template>
  <span class="text-orange-600" :contenteditable="false">
    {{ context.symbolSubtype.value ?? STATEMENT_TYPE_KEYWORD[context.statement.value.type] }}
  </span>
</template>

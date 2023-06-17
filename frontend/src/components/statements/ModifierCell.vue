<script lang="ts" setup>
import { useStatementContext } from "@/state/statement";
const context = useStatementContext();
</script>
<template>
  <span class="text-orange-600" :contenteditable="false">{{ context.statement.value.modifier }}</span>
</template>

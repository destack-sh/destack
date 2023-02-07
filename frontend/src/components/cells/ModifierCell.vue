<script lang="ts" setup>
import { useStatementContext } from "@/components/statement";
import { MODIFIER_KEYWORD } from "@/state/editor";
import { computed } from "vue";

const context = useStatementContext();
const modifierKeyword = computed(() => MODIFIER_KEYWORD[context.statement.value.modifier]);
</script>
<template>
  <span class="text-orange-600" :contenteditable="false">{{ modifierKeyword }}</span>
</template>

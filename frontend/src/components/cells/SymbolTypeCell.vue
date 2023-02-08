<script lang="ts" setup>
import { useStatementContext } from "@/components/statement";
import { TypeTag } from "@/gql/graphql";
import { SYMBOL_TYPE_KEYWORD } from "@/state/editor";
import { computed } from "vue";

const context = useStatementContext();
const symbolTypeKeyword = computed(() => {
  if (context.typeNodeRoot.value?.tag == TypeTag.Enum) {
    return "enum";
  } else {
    return SYMBOL_TYPE_KEYWORD[context.statement.value.symbolType];
  }
});
</script>
<template>
  <span class="text-orange-600" :contenteditable="false">{{ symbolTypeKeyword }}</span>
</template>

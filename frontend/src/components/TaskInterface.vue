<script lang="ts" setup>
import { graphql, useFragment, type FragmentType } from "@/gql";
import type { SymbolContentType } from "@/utils/fragments";
import { computed } from "vue";

const TaskContentType = graphql(/* GraphQL */ `
  fragment TaskContent on Task {
    id
    description
    compilations {
      id
      ...CompilationHeader
    }
  }
`);

const props = defineProps<{
  symbol: FragmentType<typeof SymbolContentType>;
  content: FragmentType<typeof TaskContentType>;
  generated: boolean;
  commented: boolean;
  focused: boolean;
  lineNumberBase: number;
  xOffset: number;
}>();
const content = computed(() => useFragment(TaskContentType, props.content));
</script>
<template>
  <div class="flex flex-col text-sm text-black">
    {{ content.description }}
  </div>
</template>

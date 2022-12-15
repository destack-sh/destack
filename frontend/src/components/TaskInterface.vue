<script lang="ts" setup>
import { graphql, useFragment, type FragmentType } from "@/gql";
import { computed } from "vue";

const TaskContent = graphql(/* GraphQL */ `
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
  symbol: unknown;
  content: FragmentType<typeof TaskContent>;
  generated: boolean;
  commented: boolean;
  focused: boolean;
  lineNumberBase: number;
  xOffset: number;
}>();
const content = computed(() => useFragment(TaskContent, props.content));
</script>
<template>
  <div class="flex flex-col text-sm text-gray-900">
    {{ content.description }}
  </div>
</template>

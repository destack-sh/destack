<script lang="ts" setup>
import { graphql, useFragment, type FragmentType } from "@/gql";
import { computed } from "vue";

const ExpectationContent = graphql(/* GraphQL */ `
  fragment ExpectationContent on Expectation {
    id
    description
  }
`);

const props = defineProps<{ content: FragmentType<typeof ExpectationContent>; focused: boolean }>();
const content = computed(() => useFragment(ExpectationContent, props.content));
</script>
<template>
  <div class="flex flex-col text-sm text-gray-900">
    {{ content.description }}
  </div>
</template>

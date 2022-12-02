<script lang="ts" setup>
import { graphql, useFragment, type FragmentType } from "@/gql";

const ExpectationContent = graphql(/* GraphQL */ `
  fragment ExpectationContent on Expectation {
    id
    description
    statements {
      id
      nameDotType
    }
  }
`);

const props = defineProps<{ content: FragmentType<typeof ExpectationContent> }>();
const content = useFragment(ExpectationContent, props.content);
</script>
<template>
  <div class="m-2 text-sm">
    {{ content.description }}
    <span v-for="statement in content.statements" :key="statement.id">
      {{ statement.nameDotType }}
    </span>
  </div>
</template>

<script lang="ts" setup>
import { graphql, useFragment, type FragmentType } from "@/gql";

const ExpectationContent = graphql(/* GraphQL */ `
  fragment ExpectationContent on Expectation {
    id
    description
    statements {
      id
      name
      typeNameDeclaration
      nameDotType
    }
  }
`);

const props = defineProps<{ content: FragmentType<typeof ExpectationContent> }>();
const content = useFragment(ExpectationContent, props.content);
</script>
<template>
  <div class="m-2 flex flex-col text-sm text-gray-900">
    {{ content.description }}
    <div class="mt-2">
      <span class="italic" v-for="statement in content.statements" :key="statement.id">
        {{ statement.typeNameDeclaration }}
      </span>
    </div>
  </div>
</template>

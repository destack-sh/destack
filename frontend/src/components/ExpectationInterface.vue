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

const props = defineProps<{ content: FragmentType<typeof ExpectationContent>; focused: boolean }>();
const content = useFragment(ExpectationContent, props.content);
</script>
<template>
  <div class="m-2 flex flex-col text-sm text-gray-900">
    {{ content.description }}
    <!-- Statements -->
    <div class="flex flex-col">
      <div class="relative" v-for="(statement, index) in content.statements" :key="statement.id">
        <span class="tracking-wide">
          {{ statement.typeNameDeclaration }}
        </span>
        <!-- Imitate Monaco line numbers -->
        <span
          class="absolute top-0 -left-12 w-6 select-none text-right font-mono text-sm"
          :class="{ 'text-orange-200': !focused, 'text-orange-400': focused }"
          >{{ index + 1 }}</span
        >
      </div>
    </div>
  </div>
</template>

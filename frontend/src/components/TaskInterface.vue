<script lang="ts" setup>
import { graphql, useFragment, type FragmentType } from "@/gql";

const TaskContent = graphql(/* GraphQL */ `
  fragment TaskContent on Task {
    id
    schema
    expectations {
      id
      description
      nameDotType
    }
    templateImplementation {
      id
      nameDotType
    }
    compilations {
      id
      ...CompilationHeader
    }
  }
`);

const CompilationHeader = graphql(/* GraphQL */ `
  fragment CompilationHeader on Compilation {
    name
    createdAt
    updatedAt
    backends {
      id
      nameDotType
    }
    targetTask {
      id
      nameDotType
    }
    targetCode {
      id
      nameDotType
    }
  }
`);

const props = defineProps<{ content: FragmentType<typeof TaskContent> }>();
const content = useFragment(TaskContent, props.content);
</script>
<template>
  <div class="m-2 text-sm">
    {{ content.schema }}
    <ul class="flex flex-col gap-2">
      <li class="flex flex-col" v-for="expectation in content.expectations" :key="expectation.id">
        <span class="text-orange-600">{{ expectation.nameDotType }}</span>
        <span>{{ expectation.description }}</span>
      </li>
    </ul>
  </div>
</template>

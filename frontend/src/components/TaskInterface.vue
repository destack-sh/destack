<script lang="ts" setup>
import { graphql, useFragment, type FragmentType } from "@/gql";
import { computed } from "vue";

const TaskContent = graphql(/* GraphQL */ `
  fragment TaskContent on Task {
    id
    inputSchema {
      ...SchemaElementContentDeep
    }
    outputSchema {
      ...SchemaElementContentDeep
    }
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

const props = defineProps<{ content: FragmentType<typeof TaskContent> }>();
const content = computed(() => useFragment(TaskContent, props.content));
</script>
<template>
  <div class="m-2 text-sm text-gray-900">
    {{ content.inputSchema }} -> {{ content.outputSchema }}
    <ul class="flex flex-col gap-2">
      <li class="flex flex-col" v-for="expectation in content.expectations" :key="expectation.id">
        <span class="text-gray-500">{{ expectation.nameDotType }}</span>
        <span>{{ expectation.description }}</span>
      </li>
    </ul>
  </div>
</template>

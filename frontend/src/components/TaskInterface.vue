<script lang="ts" setup>
import SchemaElement from "@/components/SchemaElement.vue";
import { graphql, useFragment, type FragmentType } from "@/gql";
import { SchemaElementContentDeepType } from "@/utils/fragments";
import { ArrowLongRightIcon } from "@heroicons/vue/20/solid";
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
const inputSchema = computed(() => useFragment(SchemaElementContentDeepType, content.value?.inputSchema));
const outputSchema = computed(() => useFragment(SchemaElementContentDeepType, content.value?.outputSchema));
</script>
<template>
  <div class="text-sm text-gray-900">
    <!-- Schema -->
    <div class="mx-2 my-1 flex flex-row items-center gap-1" v-if="inputSchema && outputSchema">
      <!-- Input schema -->
      <div class="flex flex-row gap-2">
        <SchemaElement v-for="element in inputSchema.elements" :key="element.id" :element="element" />
      </div>
      <!-- Nice fat arrow -->
      <ArrowLongRightIcon class="h-5 w-5 text-gray-400" />
      <!-- Output schema -->
      <div class="flex flex-row">
        <SchemaElement :element="outputSchema" />
      </div>
    </div>
    <!-- Expectations -->
    <ul class="m-2 flex flex-col gap-2">
      <li class="flex flex-col" v-for="expectation in content.expectations" :key="expectation.id">
        <span class="text-gray-500">{{ expectation.nameDotType }}</span>
        <span>{{ expectation.description }}</span>
      </li>
    </ul>
  </div>
</template>

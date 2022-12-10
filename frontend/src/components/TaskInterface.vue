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
      symbol {
        id
        name
        typeNameDeclaration
      }
      statements {
        id
        name
        typeNameDeclaration
      }
    }
    templateImplementation {
      id
      name
      typeNameDeclaration
    }
    compilations {
      id
      ...CompilationHeader
    }
  }
`);

const props = defineProps<{ content: FragmentType<typeof TaskContent>; focused: boolean }>();
const content = computed(() => useFragment(TaskContent, props.content));
const inputSchema = computed(() => useFragment(SchemaElementContentDeepType, content.value?.inputSchema));
const outputSchema = computed(() => useFragment(SchemaElementContentDeepType, content.value?.outputSchema));
</script>
<template>
  <div class="text-sm text-gray-900">
    <!-- Schema & controls -->
    <div class="mx-2 mb-2 mt-1.5 flex flex-row items-baseline justify-between">
      <!-- Controls -->
      <div class="flex flex-row items-baseline gap-2">
        <span class="text-xs font-semibold text-gray-700">main</span>
      </div>
      <!-- Schema -->
      <div class="flex flex-row items-center gap-1" v-if="inputSchema && outputSchema">
        <!-- Input schema -->
        <div class="flex flex-row gap-2">
          <SchemaElement v-for="element in inputSchema.elements" :key="element.id" :element="element" />
        </div>
        <!-- Nice fat arrow -->
        <ArrowLongRightIcon class="h-4 w-4 text-gray-400" />
        <!-- Output schema -->
        <div class="flex flex-row">
          <SchemaElement :element="outputSchema" />
        </div>
      </div>
    </div>
    <!-- Expectations -->
    <ul class="m-2 flex flex-col gap-2">
      <li class="relative flex flex-col" v-for="(expectation, index) in content.expectations" :key="expectation.id">
        <span class="tracking-wide text-gray-500">{{ expectation.symbol.typeNameDeclaration }}</span>
        <span>{{ expectation.description }}</span>
        <!-- Imitate Monaco line numbers -->
        <span
          class="absolute top-0.5 -left-12 w-6 select-none text-right font-mono text-sm"
          :class="{ 'text-orange-200': !focused, 'text-orange-400': focused }"
          >{{ index + 1 }}</span
        >
      </li>
    </ul>
  </div>
</template>

<script lang="ts" setup>
import SchemaElement from "@/components/SchemaElement.vue";
import { graphql, useFragment, type FragmentType } from "@/gql";
import { SchemaElementContentDeepType } from "@/utils/fragments";
import { ArrowLongRightIcon } from "@heroicons/vue/20/solid";
import { computed } from "vue";

const TaskContent = graphql(/* GraphQL */ `
  fragment TaskContent on Task {
    id
    description
    inputSchema {
      ...SchemaElementContentDeep
    }
    outputSchema {
      ...SchemaElementContentDeep
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
    <!-- nocheckin move schema & controls to statement meta -->
    <div v-show="false" class="mx-2 mb-2 mt-1.5 flex flex-row items-baseline justify-between">
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
  </div>
  <div class="flex flex-col text-sm text-gray-900">
    {{ content.description }}
  </div>
</template>

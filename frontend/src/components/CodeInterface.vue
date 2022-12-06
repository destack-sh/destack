<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import SchemaElement from "@/components/SchemaElement.vue";
import { graphql, useFragment, type FragmentType } from "@/gql";
import { SchemaElementContentDeepType } from "@/utils/fragments";
import { ArrowLongRightIcon } from "@heroicons/vue/20/solid";
import { computed } from "vue";

const CodeContentFragment = graphql(/* GraphQL */ `
  fragment CodeContent on Code {
    id
    builtinId
    code
    inputSchema {
      ...SchemaElementContentDeep
    }
    outputSchema {
      ...SchemaElementContentDeep
    }
    parameters {
      name
      type
      schema {
        ...SchemaElementContentDeep
      }
    }
    arguments {
      name
      type
      value
      reference {
        id
        nameDotType
      }
    }
  }
`);

const props = defineProps<{
  content: FragmentType<typeof CodeContentFragment>;
  generated: boolean;
  focused: boolean;
}>();
const content = computed(() => useFragment(CodeContentFragment, props.content));
const inputSchema = computed(() => useFragment(SchemaElementContentDeepType, content.value?.inputSchema));
const outputSchema = computed(() => useFragment(SchemaElementContentDeepType, content.value?.outputSchema));
</script>
<template>
  <div>
    <!-- Schema & controls -->
    <div class="mx-2 mb-2 mt-1.5 flex flex-row items-baseline justify-between">
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
      <!-- Controls & meta -->
      <div class="flex flex-row items-baseline gap-2">
        <span class="text-xs text-gray-700">{{ content.builtinId || "PythonX" }}</span>
      </div>
    </div>
    <MonacoEditor
      v-if="content.code"
      :model-value="content.code"
      language="python"
      :focused="focused"
      :readonly="generated"
    />
  </div>
</template>

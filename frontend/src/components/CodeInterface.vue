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
        typeNameDeclaration
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

const parameters = computed(() => content.value?.parameters ?? []);
const arguments_ = computed(() => content.value?.arguments ?? []);

function getArgument(name: string) {
  return arguments_.value?.find((a) => a.name === name);
}
</script>
<template>
  <div>
    <!-- Schema & controls -->
    <div class="mx-2 mb-2 mt-1.5 flex flex-row items-baseline justify-between">
      <!-- Controls & meta -->
      <div class="flex flex-row items-baseline gap-2">
        <span class="text-xs font-semibold text-gray-700">{{ content.builtinId || "python" }}</span>
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
    <!-- Parameters (with argument if available) -->
    <div class="m-2 flex flex-col gap-2">
      <div class="grid grid-cols-4 gap-2" v-for="parameter in parameters" :key="parameter.name">
        <div class="flex flex-row items-baseline gap-1 text-sm text-gray-900">
          <span>{{ parameter.name }}</span>
          <span class="text-gray-500">{{ parameter.type.toLowerCase() }}</span>
        </div>
        <div class="col-span-3 text-sm text-gray-900">
          <!-- Show argument if it's bound -->
          <template v-if="getArgument(parameter.name)">
            <span v-if="getArgument(parameter.name)?.value != null">
              {{ getArgument(parameter.name)?.value }}
            </span>
            <span class="italic" v-else-if="getArgument(parameter.name)?.reference != null">
              {{ getArgument(parameter.name)?.reference?.typeNameDeclaration }}
            </span>
          </template>
        </div>
      </div>
    </div>
    <MonacoEditor
      class="-mx-10"
      v-if="content.code"
      :model-value="content.code"
      language="python"
      :focused="focused"
      :readonly="generated"
    />
  </div>
</template>

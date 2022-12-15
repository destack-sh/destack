<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import { useFragment, type FragmentType } from "@/gql";
import { CodeContentType } from "@/utils/code";
import { computed } from "vue";

const props = defineProps<{
  content: FragmentType<typeof CodeContentType>;
  generated: boolean;
  commented: boolean;
  focused: boolean;
  lineNumberBase: number;
  xOffset: number;
}>();
const content = computed(() => useFragment(CodeContentType, props.content));
const parameters = computed(() => content.value?.symbol?.parameters ?? []);
const arguments_ = computed(() => content.value?.symbol?.arguments ?? []);

function getArgument(name: string) {
  return arguments_.value?.find((a) => a.name === name);
}
</script>
<template>
  <div>
    <!-- Parameters (with argument if available) -->
    <div class="flex flex-row gap-4 pb-1.5">
      <div class="flex flex-col" v-for="parameter in parameters" :key="parameter.name">
        <div class="-mb-0.5 flex flex-row items-baseline text-xs text-gray-700">
          <span>{{ parameter.name }}</span>
        </div>
        <div class="text-sm text-gray-900">
          <!-- Show argument if it's bound -->
          <template v-if="getArgument(parameter.name)">
            <span v-if="getArgument(parameter.name)?.value != null">
              {{ getArgument(parameter.name)?.value }}
            </span>
            <span class="text-black" v-else-if="getArgument(parameter.name)?.reference != null">
              {{ getArgument(parameter.name)?.reference?.typeNameDeclaration }}
            </span>
          </template>
          <!-- Otherwise show parameter type -->
          <span v-else class="text-gray-500">{{ parameter.type.toLowerCase() }}</span>
        </div>
      </div>
    </div>
    <MonacoEditor
      v-if="content.code"
      :line-number-offset="lineNumberBase + 1 /* for statement itself */"
      :line-number-shift-px="xOffset + 20"
      :style="{ marginLeft: -xOffset - 44 + 'px' }"
      :model-value="content.code"
      language="python"
      :focused="focused"
      :readonly="generated"
    />
  </div>
</template>

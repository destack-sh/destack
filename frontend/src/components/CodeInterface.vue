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
</script>
<template>
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
</template>

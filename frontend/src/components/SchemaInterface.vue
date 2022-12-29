<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import { useFragment, type FragmentType } from "@/gql";
import { SchemaContentType } from "@/state/fragments";
import { computed, ref } from "vue";

const props = defineProps<{
  content: FragmentType<typeof SchemaContentType>;
  focused: boolean;
  readonly: boolean;
  lineNumberBase: number;
  xOffset: number;
}>();
const emit = defineEmits<{
  (e: "navigateUp", position?: number): void;
  (e: "navigateDown", position?: number): void;
  (e: "escape"): void;
}>();
const content = computed(() => useFragment(SchemaContentType, props.content));

const monacoEditor = ref<InstanceType<typeof MonacoEditor> | null>(null);

defineExpose({
  focus: () => monacoEditor.value?.focus(),
  defocus: () => monacoEditor.value?.defocus(),
});
</script>
<template>
  <div class="flex flex-col w-full h-full gap-1 text-sm">
    <span v-if="content.description" class="text-black">{{ content.description }}</span>
    <MonacoEditor
      ref="monacoEditor"
      :line-number-offset="lineNumberBase + 1 /* for statement itself */"
      hide-line-numbers
      :style="{ marginLeft: -23 + 'px' }"
      :model-value="content.bsl"
      language="text/bsl"
      :focused="focused"
      :readonly="readonly"
      @navigateUp="emit('navigateUp')"
      @navigateDown="emit('navigateDown')"
      @escape="emit('escape')"
    />
  </div>
</template>

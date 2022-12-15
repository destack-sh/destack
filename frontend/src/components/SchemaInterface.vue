<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import SchemaElement from "@/components/SchemaElement.vue";
import { useFragment, type FragmentType } from "@/gql";
import { SymbolContentType } from "@/utils/fragments";
import { SchemaContentType, useSchemaInterfaceState } from "@/utils/schema";
import { computed } from "vue";

const props = defineProps<{
  symbol: FragmentType<typeof SymbolContentType>;
  content: FragmentType<typeof SchemaContentType>;
  generated: boolean;
  commented: boolean;
  focused: boolean;
  lineNumberBase: number;
  xOffset: number;
}>();
const symbol = computed(() => useFragment(SymbolContentType, props.symbol));
const content = computed(() => useFragment(SchemaContentType, props.content));

const elementAsJsonObj = computed(() => content.value?.element || {});
const elementAsJsonText = computed(() =>
  JSON.stringify(elementAsJsonObj.value, (key, value) => (value == null || key == "__typename" ? undefined : value), 2)
);

// local interface state
const state = useSchemaInterfaceState(symbol);
</script>
<template>
  <div class="flex h-full w-full flex-col gap-1 text-sm">
    <span v-if="content.description">{{ content.description }}</span>
    <MonacoEditor
      v-if="state.view == 'json'"
      :line-number-offset="lineNumberBase + 1 /* for statement itself */"
      :line-number-shift-px="xOffset + 20"
      :style="{ marginLeft: -xOffset - 44 + 'px' }"
      :model-value="elementAsJsonText"
      language="json"
      :focused="focused"
      :readonly="generated"
    />
    <SchemaElement v-else-if="state.view == 'pretty'" :element="content.element" />
  </div>
</template>

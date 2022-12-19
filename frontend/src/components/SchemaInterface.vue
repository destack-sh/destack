<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import SchemaElement from "@/components/SchemaElement.vue";
import { useFragment, type FragmentType } from "@/gql";
import { StatementContentType } from "@/utils/fragments";
import { SchemaContentType, useSchemaInterfaceState } from "@/utils/schema";
import { computed } from "vue";

const props = defineProps<{
  statement: FragmentType<typeof StatementContentType>;
  content: FragmentType<typeof SchemaContentType>;
  compiled: boolean;
  commented: boolean;
  focused: boolean;
  lineNumberBase: number;
  xOffset: number;
}>();
const emit = defineEmits<{
  (e: "navigateUp", position?: number): void;
  (e: "navigateDown", position?: number): void;
  (e: "escape"): void;
}>();
const statement = computed(() => useFragment(StatementContentType, props.statement));
const content = computed(() => useFragment(SchemaContentType, props.content));

const elementAsJsonObj = computed(() => content.value?.element || {});
const elementAsJsonText = computed(() =>
  JSON.stringify(elementAsJsonObj.value, (key, value) => (value == null || key == "__typename" ? undefined : value), 2)
);

// local interface state
const state = useSchemaInterfaceState(statement);
</script>
<template>
  <div class="flex h-full w-full flex-col gap-1 text-sm">
    <span v-if="content.description" class="text-black">{{ content.description }}</span>
    <MonacoEditor
      v-if="state.view == 'json'"
      :line-number-offset="lineNumberBase + 1 /* for statement itself */"
      :line-number-shift-px="xOffset + 20"
      :style="{ marginLeft: -xOffset - 44 + 'px' }"
      :model-value="elementAsJsonText"
      language="json"
      :focused="focused"
      :readonly="compiled"
      @navigateUp="emit('navigateUp')"
      @navigateDown="emit('navigateDown')"
      @escape="emit('escape')"
    />
    <SchemaElement
      v-else-if="state.view == 'pretty'"
      :element="content.element"
      :omit-name="content.element.name == statement.name"
    />
  </div>
</template>

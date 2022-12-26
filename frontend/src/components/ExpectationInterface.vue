<script lang="ts" setup>
import EditableSpan from "@/components/EditableSpan.vue";
import { graphql, useFragment, type FragmentType } from "@/gql";
import { StatementContentType } from "@/utils/fragments";
import { useOperations } from "@/utils/operations";
import { useDebounceFn } from "@vueuse/shared";
import { computed, ref, watchEffect, type Ref } from "vue";

const ExpectationContentType = graphql(/* GraphQL */ `
  fragment ExpectationContent on Expectation {
    id
    description
  }
`);

const props = defineProps<{
  statement: FragmentType<typeof StatementContentType>;
  content: FragmentType<typeof ExpectationContentType>;
  focused: boolean;
  editing: boolean;
  readonly: boolean;
  lineNumberBase: number;
  xOffset: number;
}>();
const emit = defineEmits<{
  (e: "navigateUp", position?: number): void;
  (e: "navigateDown", position?: number): void;
  (e: "escape"): void;
}>();

const statement = computed(() => useFragment(StatementContentType, props.statement));
const content = computed(() => useFragment(ExpectationContentType, props.content));

const operations = useOperations();
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
const description = ref("");

async function saveDescription() {
  const newDescription = description.value;
  if (newDescription.length > 0 && newDescription != content.value.description) {
    await operations.content.updateTaskContent(statement.value.id, content.value.description, newDescription);
  }
}
const saveDescriptionDebounced = useDebounceFn(saveDescription, 200, { maxWait: 500 });

// sync description to local if not editing
watchEffect(() => {
  if (!props.editing) {
    description.value = content.value.description;
  }
});

defineExpose({
  focus: () => descriptionRef.value?.focus(),
  defocus: () => descriptionRef.value?.defocus(),
});
</script>
<template>
  <div class="relative flex items-baseline text-sm text-black">
    <EditableSpan
      ref="descriptionRef"
      :readonly="readonly"
      v-model="description"
      maxlength="200"
      class="inline w-full rounded-sm bg-transparent outline-none"
      @enter="saveDescription"
      @navigateUp="emit('navigateUp')"
      @navigateDown="emit('navigateDown')"
      @escape="emit('escape')"
      @keydown="saveDescriptionDebounced"
    />
  </div>
</template>

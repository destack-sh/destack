<script lang="ts" setup>
import EditableSpan from "@/components/EditableSpan.vue";
import { useFragment, type FragmentType } from "@/gql";
import { StatementContentType } from "@/state/fragments";
import { useOperations } from "@/state/operations";
import { useDebounceFn } from "@vueuse/shared";
import { computed, ref, watchEffect, type Ref } from "vue";

const props = defineProps<{
  statement: FragmentType<typeof StatementContentType>;
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

const operations = useOperations();
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
const description: Ref<string | null> = ref(null);

async function saveDescription() {
  if (description.value != null && description.value.length > 0 && description.value != statement.value.description) {
    await operations.content.updateTaskContent(
      statement.value.id,
      statement.value.description || "",
      description.value
    );
  }
}
const saveDescriptionDebounced = useDebounceFn(saveDescription, 200, { maxWait: 500 });

// sync description to local if not editing
watchEffect(() => {
  if (!props.editing || description.value == null) {
    description.value = statement.value.description;
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
      :modelValue="description ?? ''"
      maxlength="200"
      @update:modelValue="
        description = $event;
        saveDescriptionDebounced();
      "
      @enter="saveDescription"
      class="inline w-full rounded-sm bg-transparent outline-none"
      @navigateUp="emit('navigateUp')"
      @navigateDown="emit('navigateDown')"
      @escape="emit('escape')"
    />
  </div>
</template>

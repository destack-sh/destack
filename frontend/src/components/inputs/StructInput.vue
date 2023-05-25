<script lang="ts" setup>
import { useElementRefs } from "@/composables/useGrid";
import { TypeHint, TypeTag, type SimpleType } from "@/gql/graphql";
import { symbolOf, TypeFlag } from "@/state/runtime";
import { computed, ref, type Ref } from "vue";

const props = defineProps<{
  type: SimpleType;
  modelValue: Record<string, any>[];
  readonly?: boolean;
  preview?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: Record<string, any>[]): void;
  (e: "close"): void;
}>();

const addButtonRef = ref<HTMLButtonElement | null>(null);
const structRefs = useElementRefs();

const isArray = computed(() => Boolean(props.type.flags & TypeFlag.IsArray));
const runtimeType = computed(() => symbolOf(props.type.reference?.id));

const fields = computed(() => {
  return runtimeType.value?.typeNodes ?? [];
});
const titleField: Ref<SimpleType | undefined> = computed(() => {
  // get first name or string field
  const nameField = fields.value.find((f) => f.hint == TypeHint.Name);
  if (nameField) return nameField;
  const stringField = fields.value.find((f) => f.tag == TypeTag.String);
  if (stringField) return stringField;
  return undefined;
});

function open(structIdx: number) {
  // not implemented
}

function remove(structIdx: number) {
  emit(
    "update:modelValue",
    props.modelValue.filter((_, j) => j != structIdx)
  );
}

function focus() {
  addButtonRef.value?.focus();
}
function blur() {
  addButtonRef.value?.blur();
  structRefs.refs.value.forEach((r) => r?.blur());
}

defineExpose({
  focus,
  blur,
});
</script>
<template>
  <div class="flex h-full w-full flex-row flex-wrap gap-1">
    <!-- Inline struct views -->
    <div
      v-for="(struct, i) in modelValue"
      :ref="(el: any) => structRefs.registerRef(i.toString(), el)"
      :key="i"
      class="group/struct relative flex flex-row items-center gap-1.5 bg-gray-100 px-2 hover:cursor-pointer"
      @click.stop.prevent="open(i)"
    >
      <!-- Struct title -->
      <span v-if="struct[titleField?.key ?? ''] != undefined" class="min-w-[10px] text-gray-900">{{
        struct[titleField?.key ?? ""]
      }}</span>
      <!-- default to type name if we don't have anything -->
      <span v-else class="text-gray-500 group-hover/struct:text-gray-700">{{ runtimeType?.name }}</span>
      <!-- Delete button -->
      <button
        v-if="!preview"
        class="text-gray-300 focus:text-gray-700 group-hover/struct:text-gray-500"
        @click.stop.prevent="remove(i)"
      >
        x
      </button>
      <!-- Show full struct on hover -->
      <!-- TODO @Feature: show structs properly (also needs struct interface) -->
    </div>
    <button
      v-if="!preview"
      ref="addButtonRef"
      class="self-end justify-self-end rounded-sm border-gray-300 px-0.5 transition hover:bg-orange-100 focus:bg-orange-100 focus:outline-none group-focus-within/iface:opacity-100 group-hover/iface:opacity-100"
    >
      <!-- TODO @Feature: edit structs in interface -->
      <PlusIcon class="h-4 w-4 text-gray-400" />
    </button>
    <!-- Empty content for scaling -->
    <template v-if="preview && modelValue.length == 0">&nbsp;</template>
  </div>
</template>

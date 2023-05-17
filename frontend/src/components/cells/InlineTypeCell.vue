<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import SelectTypeCell from "@/components/cells/SelectTypeCell.vue";
import SimpleTypePreview from "@/components/cells/SimpleTypePreview.vue";
import { ANY_TYPE_NODE, type SimpleType } from "@/components/statement";
import { TypeTag, type SimpleTypeNode } from "@/gql/graphql";
import { Popover, PopoverButton, PopoverPanel } from "@headlessui/vue";
import { computed, nextTick, ref, watch, type Ref } from "vue";

const props = defineProps<{
  modelValue?: SimpleType;
  readonly: boolean;
  inlined?: boolean;
  structrefOnly?: boolean;
  hideFlags?: boolean;
  hideIcon?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: Pick<SimpleType, "name" | "tag" | "flags" | "reference">): void;
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "navigateLeft"): void;
  (e: "navigateRight"): void;
  (e: "deleteLeft"): void;
  (e: "deleteSelf"): void;
  (e: "enter"): void;
  (e: "escape"): void;
  (e: "focus", event: FocusEvent): void;
}>();

const value: Ref<SimpleType> = ref(props.modelValue ?? ANY_TYPE_NODE);
const buttonRef: Ref<HTMLButtonElement | null> = ref(null);
const valueRef: Ref<InstanceType<typeof SelectTypeCell> | null> = ref(null);
const popoverButtonRef: Ref<InstanceType<typeof PopoverButton> | null> = ref(null);

const popoverOpenRef: Ref<HTMLSpanElement | null> = ref(null);

function writeValue(type: SimpleTypeNode) {
  nextTick(() => buttonRef.value?.focus());
  // keep flags (they're configured in a separate interface)
  type = {
    ...type,
    flags: value.value.flags,
  };
  value.value = type;
  emit("update:modelValue", type);
  emit("escape");
}

// sync modelValue if changed externally
watch(
  () => [props.modelValue],
  () => {
    if (props.modelValue != value.value) {
      value.value = props.modelValue ?? ANY_TYPE_NODE;
    }
  }
);

function open() {
  if (popoverOpenRef.value == null) {
    popoverButtonRef.value?.$el.click();
    nextTick(() => valueRef.value?.focus());
  }
}

function focus() {
  buttonRef.value?.focus();
}

function blur() {
  buttonRef.value?.blur();
}

defineExpose({
  editing: computed(() => popoverOpenRef.value != null),
  focus,
  blur,
});
</script>
<template>
  <Popover as="div" v-slot="{ close }" class="relative">
    <!-- Type preview -->
    <button
      ref="buttonRef"
      tabindex="-1"
      :disabled="readonly"
      @keydown.left.exact.prevent="emit('navigateLeft')"
      @keydown.right.exact.prevent="emit('navigateRight')"
      @keydown.up.exact.prevent="emit('navigateUp')"
      @keydown.down.exact.prevent="emit('navigateDown')"
      @keydown.delete.exact="editing || emit('deleteSelf')"
      class="h-full w-full text-left outline-none"
      @click="open"
      @keydown.enter.exact.prevent="open"
    >
      <SimpleTypePreview :type="value" :hide-icon="hideIcon || value.reference != null" />
    </button>
    <PopoverButton ref="popoverButtonRef" @focus.prevent="focus" class="hidden" />
    <!-- Editable type :EditableCellStyle -->
    <FadeTransition>
      <PopoverPanel
        class="absolute -left-2 z-10 flex flex-col gap-2 rounded-sm bg-white p-1 shadow-md ring-1 ring-orange-900 ring-opacity-40"
        :class="hideFlags ? '-top-2' : '-top-10'"
        unmount
      >
        <span ref="popoverOpenRef" class="hidden" />
        <SelectTypeCell
          ref="valueRef"
          as="div"
          :model-value="value"
          @update:model-value="writeValue($event), close(), buttonRef?.focus()"
          @escape="close(), buttonRef?.focus()"
          :inlined="inlined"
          :structref-only="structrefOnly"
          :hide-flags="hideFlags"
        />
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>

<script lang="ts" setup>
import { useElementRefs } from "@/composables/useGrid";
import type { Field } from "@/state/module";
import { computed, ref, type Ref } from "vue";
import TypeInterface from "@/components/interfaces/TypeInterface.vue";
import { ArrowRightIcon } from "@heroicons/vue/24/outline";
import { TypeTag } from "@/gql/graphql";

const context = useStatementContext();

const baseTypesRefs = useElementRefs<InstanceType<typeof TypeInterface>>();
const baseTypes = computed(() => context.baseTypes.value);

const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "addBase"): void;
}>();

defineExpose({
  focus: () => {
    declarationRef.value?.focus();
  },
  focusLastBase: () => {
    baseTypesRefs.focus(baseTypes.value[baseTypes.value.length - 1].id);
  },
  blur: () => {
    declarationRef.value?.blur();
    baseTypesRefs.refs.value.forEach((r) => r.blur?.());
    extendButtonRef.value?.blur();
  },
});
</script>
<template>
  <!-- Base types -->
  <div class="ml-1 whitespace-nowrap" v-if="(baseTypes.length ?? 0) > 0">
    <ArrowRightIcon class="mb-0.5 mr-1 inline-block h-4 w-4 text-orange-600" />
    <div class="inline-flex flex-row gap-x-1">
      <TypeInterface
        v-for="field of baseTypes"
        :ref="(el: any) => baseTypesRefs.registerRef(field.id, el)"
        :model-value="field"
        @update:model-value="(val) => context.updateField(field, val as Field)"
        @delete-self="context.deleteField(field)"
        @navigate-left="
          field.id == baseTypes[0].id
            ? declarationRef?.focus()
            : baseTypesRefs.focus(baseTypes[baseTypes.findIndex((n) => n.id == field.id) - 1].id)
        "
        @navigate-right="
          field.id == baseTypes[baseTypes.length - 1].id
            ? extendButtonRef?.focus()
            : baseTypesRefs.focus(baseTypes[baseTypes.findIndex((n) => n.id == field.id) + 1].id)
        "
        @navigate-down="emit('navigateDown')"
        @navigate-up="emit('navigateUp')"
        :active="context.focused.value || context.editing.value"
        :key="field.id"
        :readonly="context.readonly.value"
        ref-only
        :ref-types="[TypeTag.Struct, TypeTag.Function]"
        hide-flags
        class="w-full rounded-sm border border-transparent border-opacity-[15%] text-orange-600 focus-within:border-solid focus-within:border-orange-900 focus-within:bg-orange-100 hover:bg-orange-100"
      />
    </div>
  </div>
  <!-- Inline buttons -->
  <!-- (removed because they looked cluttered) -->
</template>

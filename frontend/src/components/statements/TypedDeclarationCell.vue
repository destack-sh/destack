<script lang="ts" setup>
import { useElementRefs } from "@/composables/useGrid";
import { useStatementContext } from "@/state/statement";
import { computed, ref, type Ref } from "vue";
import DeclarationCell from "@/components/statements/DeclarationCell.vue";
import TypeInterface from "@/components/interfaces/TypeInterface.vue";

const context = useStatementContext();

const declarationRef: Ref<InstanceType<typeof DeclarationCell> | null> = ref(null);
const baseTypesRefs = useElementRefs<InstanceType<typeof TypeInterface>>();
const baseTypes = computed(() => context.baseTypes.value);
const extendButtonRef: Ref<HTMLButtonElement | null> = ref(null);

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
  <DeclarationCell ref="declarationRef" @navigate-up="emit('navigateUp')" @navigate-down="emit('navigateDown')" />
  <!-- Base types -->
  <div class="ml-1 whitespace-nowrap" v-if="(baseTypes.length ?? 0) > 0">
    <span class="mr-1 text-orange-600">has</span>
    <div class="inline-flex flex-row gap-x-1">
      <TypeInterface
        v-for="field of baseTypes"
        :ref="(el: any) => baseTypesRefs.registerRef(field.id, el)"
        :model-value="field"
        @update:model-value="(val) => context.updateField(field, val)"
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
        structref-only
        hide-flags
        hide-icon
        class="w-full rounded-sm border border-transparent border-opacity-[15%] focus-within:border-solid focus-within:border-orange-900 focus-within:bg-orange-100 hover:bg-orange-100"
      />
    </div>
  </div>
  <!-- Inline buttons -->
  <!-- (removed because they looked cluttered) -->
</template>

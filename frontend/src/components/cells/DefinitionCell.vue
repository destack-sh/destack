<script lang="ts" setup>
import BlankCell from "@/components/cells/BlankCell.vue";
import ModifierCell from "@/components/cells/ModifierCell.vue";
import { useStatementContext } from "@/components/statement";
import { ref, type Ref } from "vue";

const context = useStatementContext();

const name: Ref<string> = ref(context.statement.value.name ?? "");
context.syncName(name);

const gapRef: Ref<InstanceType<typeof BlankCell> | null> = ref(null);
</script>
<template>
  <div class="flex flex-row flex-wrap">
    <ModifierCell v-if="context.statement.value.modifier" />
    <BlankCell ref="gapRef" @navigate-up="context.navigateUp" @delete-left="context.morphSetModifier(null)" />
    <SymbolTypeCell />
    <EditableSpan ref="nameRef" />
  </div>
</template>

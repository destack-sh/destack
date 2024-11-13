<script lang="ts" setup>
import { ViewData, NodeType } from "@/proto/wire";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { toRef } from "vue";
import { TypedNodeReferenceData } from "@/proto/wiring";

const props = defineProps<
	{ self: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Pick<ViewData, "name" | "title" | "icon" | "nodePtr">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

defineExpose<ViewExposed>({ self });
</script>
<template>
	<div>
		nocheckin: Help
	</div>
</template>
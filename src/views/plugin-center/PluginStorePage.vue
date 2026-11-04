<script setup lang="ts">
import ToolCatalogGrid from "@/components/ToolCatalogGrid.vue";
defineProps<{ detached: boolean }>();
const emit = defineEmits<{ detail: [name: string] }>();
</script>

<template>
  <section class="pc__page">
    <ToolCatalogGrid :shared-search="!detached" @detail="emit('detail', $event)" />
  </section>
</template>

<style scoped src="./plugin-center.css"></style>

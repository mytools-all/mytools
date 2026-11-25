<script setup lang="ts">
import MyPanelView from "@/views/MyPanelView.vue";
import { useWindowLifecycle } from "@/composables/useWindowLifecycle";
import { useSettingsStore } from "@/stores/settings";
const settings = useSettingsStore();
useWindowLifecycle({ onFocusGained: () => { void settings.init(); } });
</script>
<template><MyPanelView /></template>

<script setup lang="ts">
import {computed,ref} from 'vue';
import {formatBytes,privateMemoryLabel,type ApplicationRow,type ProcessRow} from '@/lib/resources';
const props=defineProps<{groups?:ApplicationRow[];processes:ProcessRow[];warming:boolean}>();
const expanded=ref(new Set<string>());
const rows=computed(()=>props.groups??props.processes.map(p=>({id:String(p.pid),name:p.name,rootPid:p.pid,processCount:1,cpu:p.cpu,memory:p.memory,privateMemory:p.privateMemory??null,knownPrivateMemory:p.privateMemory??0,privateMemoryKnown:p.privateMemory==null?0:1,association:'single-process',processes:[p]})));
function toggle(id:string){const next=new Set(expanded.value);if(!next.delete(id))next.add(id);expanded.value=next;}
</script>
<template>
  <h3>外部服务</h3>
  <p class="note">独立于 MyTools 统计。相同程序的服务与客户端经系统配置核实后合并，可展开查看 PID。私有工作集不含可共享页；读取失败明确标注，服务接口是否可用仍需单独确认。</p>
  <table>
    <thead><tr><th>应用 / 服务</th><th>进程数</th><th>CPU</th><th>私有工作集</th><th>工作集（含共享）</th></tr></thead>
    <tbody>
      <template v-for="group in rows" :key="group.id">
        <tr><td><button class="group-toggle" :aria-expanded="expanded.has(group.id)" @click="toggle(group.id)">{{ group.name }}（{{ group.processCount }} 个进程）</button></td><td>{{ group.processCount }}</td><td>{{ warming?'采样中':group.cpu.toFixed(1)+'%' }}</td><td>{{ privateMemoryLabel(group) }}</td><td>{{ formatBytes(group.memory) }}</td></tr>
        <template v-if="expanded.has(group.id)">
          <tr v-for="p in group.processes" :key="p.pid" class="child"><td>{{ p.name }}<small>{{ p.role||'进程' }}</small></td><td>PID {{ p.pid }}</td><td>{{ warming?'采样中':p.cpu.toFixed(1)+'%' }}</td><td>{{ privateMemoryLabel(p) }}</td><td>{{ formatBytes(p.memory) }}</td></tr>
        </template>
      </template>
    </tbody>
  </table>
  <p v-if="!rows.length">未检测到已知外部服务进程。</p>
</template>
<style scoped>
.note,p,small{font-size:12px;color:var(--text-dim);line-height:1.7}table{width:100%;border-collapse:collapse;font-size:12px}th,td{padding:10px 7px;border-bottom:1px solid var(--panel-border);text-align:left;overflow-wrap:anywhere}small{display:block}.child{background:var(--surface-bg)}.child td:first-child{padding-left:20px}.group-toggle{color:var(--accent);border:1px solid transparent;background:transparent;padding:5px;text-align:left;border-radius:6px;cursor:pointer}.group-toggle:focus-visible{outline:2px solid var(--accent)}
</style>

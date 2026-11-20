import {mount,flushPromises} from '@vue/test-utils';
import {defineComponent,KeepAlive,nextTick,ref} from 'vue';
import {afterEach,expect,it,vi} from 'vitest';
import ResourceManagerPage from './ResourceManagerPage.vue';
import {sizeLabel,formatBytes} from '@/lib/resources';
const invoke=vi.hoisted(()=>vi.fn(async(cmd:string)=>cmd==='resources_storage'?{plugins:[],categories:[],sharedStoreReadable:true}:null));
vi.mock('@/lib/tauri',()=>({tauriAvailable:()=>true,safeInvoke:invoke}));
afterEach(()=>{vi.useRealTimers();invoke.mockClear();});
it('stops sampling on deactivation and unmount, then resumes only once',async()=>{
 vi.useFakeTimers();
 const visible=ref(true);
 const wrapper=mount(defineComponent({components:{ResourceManagerPage,KeepAlive},setup:()=>({visible}),template:'<KeepAlive><ResourceManagerPage v-if="visible" /></KeepAlive>'}));
 await flushPromises();
 const samples=()=>invoke.mock.calls.filter(([cmd])=>cmd==='resources_snapshot').length;
 expect(samples()).toBe(1);
 await vi.advanceTimersByTimeAsync(2000);expect(samples()).toBe(2);
 visible.value=false;await nextTick();await vi.advanceTimersByTimeAsync(6000);expect(samples()).toBe(2);
 visible.value=true;await nextTick();await flushPromises();expect(samples()).toBe(3);
 await vi.advanceTimersByTimeAsync(2000);expect(samples()).toBe(4);
 wrapper.unmount();await vi.advanceTimersByTimeAsync(4000);expect(samples()).toBe(4);
});
it('distinguishes unavailable, missing and partial directory sizes',()=>{
 const size={bytes:1024,files:1,exists:true,limited:false,errors:0,skipped:0};
 expect(sizeLabel(size)).toBe('1.0 KiB');
 expect(sizeLabel({...size,skipped:1})).toContain('部分扫描');
 expect(sizeLabel({...size,limited:true})).toContain('部分扫描');
 expect(sizeLabel({...size,exists:false})).toBe('未创建');
 expect(formatBytes(null)).toBe('不可用');
});

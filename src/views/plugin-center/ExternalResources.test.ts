import {mount} from '@vue/test-utils';
import {expect,it} from 'vitest';
import ExternalResources from './ExternalResources.vue';
it('groups service and client, reveals distinct PIDs and marks partial private memory',async()=>{
 const wrapper=mount(ExternalResources,{props:{warming:false,processes:[],groups:[{id:'service:1',name:'Everything.exe',rootPid:1,processCount:2,cpu:2,memory:3000,privateMemory:null,knownPrivateMemory:500,privateMemoryKnown:1,association:'verified-service-executable',processes:[{pid:1,parent:null,name:'Everything.exe',cpu:1,memory:1000,privateMemory:null,role:'Windows 服务：Everything'},{pid:2,parent:null,name:'Everything.exe',cpu:1,memory:2000,privateMemory:500,role:'客户端'}]}]}});
 expect(wrapper.findAll('tbody tr')).toHaveLength(1);
 expect(wrapper.text()).toContain('部分 1/2');
 await wrapper.get('button').trigger('click');
 expect(wrapper.findAll('tbody tr')).toHaveLength(3);
 expect(wrapper.text()).toContain('Windows 服务：Everything');
 expect(wrapper.text()).toContain('客户端');
 expect(wrapper.text()).toContain('PID 1');
 expect(wrapper.text()).toContain('PID 2');
 expect(wrapper.text()).toContain('不可用');
 wrapper.unmount();
});

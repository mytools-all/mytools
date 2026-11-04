import {readFile} from 'node:fs/promises';
import {dirname,join,resolve} from 'node:path';
import {pathToFileURL} from 'node:url';

const system=new Set('advapi32 bcrypt bcryptprimitives cabinet cfgmgr32 comctl32 comdlg32 combase crypt32 cryptbase d2d1 d3d11 d3d12 dcomp dbghelp dnsapi dwmapi dwrite dxgi gdi32 imm32 iphlpapi kernel32 kernelbase mpr msvcrt ncrypt netapi32 normaliz ntdll ole32 oleaut32 pdh powrprof propsys psapi rpcrt4 secur32 setupapi shell32 shlwapi user32 userenv uxtheme version winhttp wininet winmm winspool wintrust ws2_32 wtsapi32'.split(' ').map(name=>name+'.dll'));
export function peImports(bytes){
  if(bytes.length<64 || bytes.toString('ascii',0,2)!=='MZ')throw Error('Not a PE executable');
  const pe=bytes.readUInt32LE(60);if(pe+24>bytes.length || bytes.toString('ascii',pe,pe+4)!=='PE\0\0')throw Error('Invalid PE header');
  const count=bytes.readUInt16LE(pe+6),size=bytes.readUInt16LE(pe+20),optional=pe+24;
  if(optional+size>bytes.length || count>96)throw Error('Truncated PE header');
  const magic=bytes.readUInt16LE(optional),directory=magic===0x20b?112:magic===0x10b?96:0;
  if(!directory || size<directory+16)throw Error('Unsupported PE optional header');
  const rva=bytes.readUInt32LE(optional+directory+8);if(!rva)return [];
  const section=optional+size;
  if(section+count*40>bytes.length)throw Error('Truncated PE sections');
  const offset=value=>{
    for(let i=0;i<count;i++){
      const at=section+i*40,virtual=bytes.readUInt32LE(at+12),length=Math.max(bytes.readUInt32LE(at+8),bytes.readUInt32LE(at+16));
      if(value>=virtual && value<virtual+length){const result=bytes.readUInt32LE(at+20)+value-virtual;if(result>=bytes.length)break;return result;}
    }
    throw Error('Invalid PE import address');
  };
  const result=[];let at=offset(rva);
  for(let i=0;i<1024;i++,at+=20){
    if(at+20>bytes.length)throw Error('Truncated PE imports');
    if(bytes.subarray(at,at+20).every(value=>value===0))return [...new Set(result)];
    const start=offset(bytes.readUInt32LE(at+12)),end=bytes.indexOf(0,start);
    if(end<start || end-start>260)throw Error('Invalid PE import name');
    const name=bytes.toString('ascii',start,end);if(!/^[a-z0-9_.-]+\.dll$/i.test(name))throw Error('Unsafe PE import name');
    result.push(name);
  }
  throw Error('Too many PE imports');
}
export async function checkRuntime(executable){
  const folder=dirname(resolve(executable)),visited=new Set();
  async function check(file){
    const key=file.toLowerCase();if(visited.has(key))return;visited.add(key);
    const imports=peImports(await readFile(file));
    for(const name of imports){
      const lower=name.toLowerCase();if(system.has(lower)||lower.startsWith('api-ms-win-')||lower.startsWith('ext-ms-win-'))continue;
      const dependency=join(folder,name);
      try{await check(dependency);}catch(error){throw Error(`Missing or invalid runtime dependency ${name} required by ${file}: ${error.message}`);}
    }
  }
  await check(resolve(executable));return [...visited];
}
if(process.argv[1] && pathToFileURL(resolve(process.argv[1])).href===import.meta.url){
  if(!process.argv[2])throw Error('Usage: node scripts/check-runtime-deps.mjs <mytools.exe>');
  console.log('Runtime dependency check passed:',(await checkRuntime(process.argv[2])).length,'packaged PE files');
}

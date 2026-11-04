import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,dirname,basename,resolve} from 'node:path';
import {peImports,checkRuntime} from './check-runtime-deps.mjs';
function fixture(names){
 const b=Buffer.alloc(2048);b.write('MZ');b.writeUInt32LE(128,60);b.write('PE\0\0',128);b.writeUInt16LE(1,134);b.writeUInt16LE(240,148);b.writeUInt16LE(0x20b,152);b.writeUInt32LE(0x1000,272);
 const section=392;b.writeUInt32LE(1536,section+8);b.writeUInt32LE(0x1000,section+12);b.writeUInt32LE(1536,section+16);b.writeUInt32LE(512,section+20);
 names.forEach((name,i)=>{const start=1024+i*100;b.writeUInt32LE(0x1000+start-512,512+i*20+12);b.write(name+'\0',start);});return b;
}
test('reads actual PE import table layout and rejects malformed data',()=>{assert.deepEqual(peImports(fixture(['KERNEL32.dll','liblzma-5.dll'])),['KERNEL32.dll','liblzma-5.dll']);assert.throws(()=>peImports(Buffer.from('not a PE')));});
test('rejects missing development DLLs and follows packaged dependencies',async t=>{
 const dir=await mkdtemp(join(tmpdir(),'mytools-runtime-'));assert.equal(dirname(resolve(dir)),resolve(tmpdir()));assert.ok(basename(dir).startsWith('mytools-runtime-'));t.after(()=>rm(dir,{recursive:true,force:true}));const executable=join(dir,'mytools.exe');await writeFile(executable,fixture(['KERNEL32.dll','liblzma-5.dll']));await assert.rejects(checkRuntime(executable),/liblzma-5.dll/);await writeFile(join(dir,'liblzma-5.dll'),fixture(['KERNEL32.dll']));assert.equal((await checkRuntime(executable)).length,2);
});

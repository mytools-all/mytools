import test from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';

test('driver compatibility uses the WebView2 build rather than the installed Edge major',{skip:process.platform!=='win32'},()=>{
  const script=fileURLToPath(new URL('./webview2-runtime.ps1',import.meta.url)).replaceAll("'","''");
  const result=spawnSync('pwsh',['-NoProfile','-Command',`. '${script}'; @((Test-WebViewDriverVersion '131.0.2903.86' '156.0.4314.58'),(Test-WebViewDriverVersion '131.0.2903.86' '131.0.2903.112'),(Test-WebViewDriverVersion '131.0.2903.86' '131.0.2904.86'),(Test-WebViewDriverVersion 'invalid' '131.0.2903.86')) | ConvertTo-Json -Compress`],{encoding:'utf8'});
  assert.equal(result.status,0,result.stderr);
  assert.deepEqual(JSON.parse(result.stdout),[false,true,false,false]);
});

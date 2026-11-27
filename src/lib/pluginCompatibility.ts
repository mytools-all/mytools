import satisfies from "semver/functions/satisfies";
import validRange from "semver/ranges/valid";
import minVersion from "semver/ranges/min-version";
import lt from "semver/functions/lt";
import { APP_INFO } from "@/config/app";

export function pluginCompatibility(value: { engines?: Record<string,string>; apiVersion?: number }, host=APP_INFO.version): {compatible:boolean;declared:boolean;message:string;action?:"host"|"plugin"} {
  const range=value.engines?.mytools;
  if(value.apiVersion!==undefined&&value.apiVersion>1)return{compatible:false,declared:true,message:`需要插件 API v${value.apiVersion}，当前提供 v1，请检查 MyTools 更新`,action:"host"};
  if(range===undefined)return{compatible:true,declared:false,message:"未声明 MyTools 版本范围"};
  if(range.length>512||!range.trim()||!validRange(range))return{compatible:false,declared:true,message:`MyTools 版本范围无效：${range.slice(0,512)}`};
  const compatible=satisfies(host,range);
  if(compatible)return{compatible:true,declared:true,message:`支持 MyTools ${range} · 当前 ${host}`};
  const minimum=minVersion(range);
  const below=minimum!==null&&lt(host,minimum);
  return{compatible:false,declared:true,action:below?"host":"plugin",message:`此插件支持 MyTools ${range}，当前为 ${host}。${below?"请升级到支持的 MyTools 版本后安装。":"当前版本不在支持范围内，请检查是否有兼容的插件新版本。"}`};
}

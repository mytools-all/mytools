import satisfies from "semver/functions/satisfies";
import validRange from "semver/ranges/valid";
import { APP_INFO } from "@/config/app";

export function pluginCompatibility(value: { engines?: Record<string,string>; apiVersion?: number }, host=APP_INFO.version): {compatible:boolean;declared:boolean;message:string} {
  const range=value.engines?.mytools;
  if(value.apiVersion!==undefined&&value.apiVersion>1)return{compatible:false,declared:true,message:`需要插件 API v${value.apiVersion}，当前提供 v1`};
  if(range===undefined)return{compatible:true,declared:false,message:"未声明 MyTools 版本范围"};
  if(range.length>512||!range.trim()||!validRange(range))return{compatible:false,declared:true,message:`MyTools 版本范围无效：${range.slice(0,512)}`};
  const compatible=satisfies(host,range);
  return{compatible,declared:true,message:`支持 MyTools ${range}${compatible?"":"，当前为 "+host}`};
}

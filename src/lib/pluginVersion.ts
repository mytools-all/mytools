function parse(value: string): string[] | null {
  const match=/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+[0-9A-Za-z.-]+)?$/.exec(value);
  if(!match || match[4]?.split(".").some(part=>/^0\d+$/.test(part))) return null;
  return [match[1]!,match[2]!,match[3]!,match[4] || ""];
}
function numberOrder(a:string,b:string) { return a.length!==b.length ? Math.sign(a.length-b.length) : a===b?0:a>b?1:-1; }
export function comparePluginVersions(a:string,b:string):number|null {
  const left=parse(a),right=parse(b);if(!left||!right)return null;
  for(let index=0;index<3;index++){const order=numberOrder(left[index]!,right[index]!);if(order)return order;}
  if(!left[3]||!right[3])return left[3]===right[3]?0:left[3]?-1:1;
  const x=left[3].split("."),y=right[3].split(".");
  for(let index=0;index<Math.max(x.length,y.length);index++){
    const p=x[index],q=y[index];if(p===undefined||q===undefined)return p===q?0:p===undefined?-1:1;
    if(p===q)continue;const pn=/^\d+$/.test(p),qn=/^\d+$/.test(q);
    return pn&&qn?numberOrder(p,q):pn!==qn?pn?-1:1:p>q?1:-1;
  }
  return 0;
}

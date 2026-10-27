// Runs the untouched whole source in a closed CommonJS VM. Host I/O exists only
// in this harness, outside the context, for reading own pinned files/writing evidence.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const root=path.resolve(__dirname,'../..'),base=root+'/v2/.qa/v1',ts=require(base+'/node_modules/typescript');
const deny=(name)=>()=>{throw Error('DENIED boundary: '+name)};
let hooks=[],cursor=0,effects=[],dirty=false,storage=new Map(),requests=[],toasts=[],copies=[],navigation=[],prefs={provider:'exchangerate-api',api_key:'owned-fixture-key'},clock=1735689600000,logs=0;
const fixture={status:200,body:{result:'success',base_code:'USD',conversion_rates:{USD:1,JPY:150,EUR:0.9,GBP:0.8,AED:3.67},time_last_update_unix:1735689600,time_next_update_unix:1735776000,time_last_update_utc:'Wed, 01 Jan 2025 00:00:00 GMT'}};
let React;
const same=(a,b)=>a&&b&&a.length===b.length&&a.every((v,i)=>Object.is(v,b[i]));
React={useState(initial){const i=cursor++;if(!(i in hooks))hooks[i]=typeof initial==='function'?initial():initial;return [hooks[i],(v)=>{hooks[i]=typeof v==='function'?v(hooks[i]):v;dirty=true}]},useRef(initial){const i=cursor++;return hooks[i]??=( {current:initial})},useEffect(fn,deps){const i=cursor++;if(!same(hooks[i],deps)){hooks[i]=deps;effects.push(fn)}},useCallback(fn,deps){const i=cursor++;if(!hooks[i]||!same(hooks[i].deps,deps))hooks[i]={fn,deps};return hooks[i].fn}};
const jsx=(type,props)=>({type,props:props||{}}),component=(n)=>n;
const api={Icon:new Proxy({}, {get:(_,k)=>'Icon.'+String(k)}),List:Object.assign(component('List'),{}),Action:{CopyToClipboard:'Action.CopyToClipboard',SubmitForm:'Action.SubmitForm'},ActionPanel:'ActionPanel',getPreferenceValues:()=>({...prefs}),Toast:{Style:{Failure:'failure'}},showToast:async(v)=>{toasts.push(v)},LocalStorage:{getItem:async(k)=>storage.get(k),setItem:async(k,v)=>{storage.set(k,v)}},Form:{DatePicker:{Type:{Date:'Date'}}},useNavigation:()=>({push:x=>navigation.push(x),pop:()=>navigation.pop()})};
// Component tokens with property support, no application APIs.
api.List={Dropdown:{Item:'List.Dropdown.Item'},Section:'List.Section',Item:'List.Item',EmptyView:'List.EmptyView'};
const nodeTokens=new Map([[api.List,'List'],[api.List.Dropdown,'List.Dropdown'],[api.Form,'Form'],[api.Form.DatePicker,'Form.DatePicker'],[api.Action,'Action']]);
const NativeDate=Date;class FakeDate extends NativeDate{constructor(...a){super(...(a.length?a:[clock]))}static now(){return clock}}
const globals={console:{log:()=>logs++,error:()=>logs++,warn:()=>logs++},Date:FakeDate,Intl,AbortController,URL,URLSearchParams,TextEncoder,TextDecoder,Buffer:require('node:buffer').Buffer,ReadableStream,WritableStream,TransformStream,DOMException,queueMicrotask,setTimeout:deny('timer'),setInterval:deny('interval'),process:Object.freeze({version:'v18.20.0',versions:Object.freeze({node:'18.20.0'}),nextTick:queueMicrotask,emitWarning:()=>logs++})};
const context=vm.createContext(globals,{codeGeneration:{strings:false,wasm:false}}),cache=new Map(),loaded=new Set();
function native(name){
 if(name==='node:http'||name==='node:https')return {validateHeaderName:require('node:http').validateHeaderName,validateHeaderValue:require('node:http').validateHeaderValue,request:deny(name+'.request'),get:deny(name+'.get')};
 const pure=['node:buffer','node:stream','node:stream/web','node:util','node:url','node:events','node:path','node:assert','node:net'];
 if(name==='node:net')return {isIP:require('node:net').isIP};
 if(pure.includes(name))return require(name);
 return new Proxy({__esModule:false},{get:(o,k)=>k in o?o[k]:deny(name+'.'+String(k))});
}
function load(file){
 if(cache.has(file))return cache.get(file).exports;
 assert(file.startsWith(base+'/')||file.startsWith(root+'/src/'),file);
 const mod={exports:{}};cache.set(file,mod);loaded.add(path.relative(base,file));
 let code=fs.readFileSync(file,'utf8');if(file.endsWith('.json')){mod.exports=JSON.parse(code);return mod.exports}
 const opts={module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022,jsx:ts.JsxEmit.ReactJSX,esModuleInterop:true,allowJs:true};
 code=ts.transpileModule(code,{compilerOptions:opts,fileName:file}).outputText;
 if(file===root+'/src/index.tsx') code+='\nObject.assign(exports,{performExchange,currencyAPI,exchangeRateAPI,uniRateAPI,enrichExchangeData,SetCurrencyDateForm,ExchangeResultActionPanel,getFlagEmoji,formatCurrency});';
 const local=(id)=>{
  if(id==='react')return React;if(id==='react/jsx-runtime')return {jsx,jsxs:jsx,Fragment:'Fragment'};if(id==='@raycast/api')return api;
  if(id.startsWith('node:')||['fs','os','process','http','https','child_process','worker_threads','net'].includes(id))return native(id.startsWith('node:')?id:'node:'+id);
  let target;
  if(id.startsWith('.')){const p=path.resolve(path.dirname(file),id);target=[p,p+'.ts',p+'.tsx',p+'.js',p+'/index.js'].find(p=>fs.existsSync(p)&&fs.statSync(p).isFile());}
  else target=require.resolve(id,{paths:[path.dirname(file),base]});
  const e=load(target);
  if(id==='node-fetch')return {...e,__esModule:true,default:async(url,options)=>{assert(options?.signal);assert.equal(options.method,'GET');const u=new URL(url);assert(['v6.exchangerate-api.com','api.unirateapi.com'].includes(u.hostname));requests.push({origin:u.origin,path:u.pathname.replace(prefs.api_key,'<key>'),query:[...u.searchParams].map(([k,v])=>[k,k==='api_key'?'<key>':v]),headers:options.headers});return new e.Response(JSON.stringify(fixture.body),{status:fixture.status})}};
  return e;
 };
 new vm.Script('(function(require,module,exports){'+code+'\n})',{filename:file}).runInContext(context)(local,mod,mod.exports);return mod.exports;
}
const helper=load(root+'/src/lib/currency.ts'),source=load(root+'/src/index.tsx'),tables=load(root+'/src/currency.tsx');
function expand(element){if(!element||typeof element!=='object')return element;if(Array.isArray(element))return element.map(expand);let {type,props}=element;if(typeof type==='function')return expand(type(props));return {type:nodeTokens.get(type)||type,props,children:[props?.children].flat(3).filter(x=>x!=null).map(expand)}}
function render(){cursor=0;dirty=false;return expand(source.default())}
const pause=()=>new Promise(resolve=>setImmediate(resolve));
async function settle(){let tree,stable=0;for(let i=0;i<50;i++){tree=render();for(const effect of effects.splice(0))effect();await pause();if(!dirty&&!effects.length){if(++stable>=6)return tree;}else stable=0;}throw Error('unsettled source hooks')}
function walk(tree,type){if(!tree||typeof tree!=='object')return [];const more=[...(tree.children||[])];for(const k of ['searchBarAccessory','actions'])if(tree.props?.[k])more.push(expand(tree.props[k]));return [tree,...more.flatMap(x=>walk(x,type))].filter(x=>x?.type===type)}
function num(n){return {raw:String(n),negativeZero:Object.is(n,-0)}}
async function main(){
 // Boundary probes run through the same VM require and globals.
 assert.throws(()=>new vm.Script('process.cwd()').runInContext(context));assert.throws(()=>native('node:fs').readFileSync('/etc/passwd'));assert.throws(()=>native('node:https').get('https://example.invalid'));assert.throws(()=>new vm.Script('setTimeout(()=>{},1)').runInContext(context));
 const expressions=['','10','1+2 in EUR','5.5 in USD at 2022/9/25','10 in GBP at 2023/01/15',' 10','10 + 2 in japan','10 in euro at 2023/2/31','10 at 2023/13/1','10\nin JPY','1e3','1..2 in usd','2^3^2','-2^2','2^-2','(-2)^3','0/0','1/0','0','-0','-.5','.5','1.','1+-2','1--2','1 in at 2024/1/1','1 in japan at 2024/1/1','1 in USD at 0001/1/1','1 at 0050/1/1','1 at 2024/4/31','1 at 2024/1/32'];
 let seed=1234567;const rand=()=>((seed=(1664525*seed+1013904223)>>>0)/2**32);
 for(let i=0;i<2000;i++){const a=Math.floor(rand()*1000),b=Math.floor(rand()*1000)+1,c=Math.floor(rand()*100)+1;expressions.push(`(${a}${['+','-','*','/'][i%4]}${b})/${c}${i%3?'':' in eur at 2023/2/31'}`)}
 for(let i=1;i<=160;i++)expressions.push(`(${i}/7)^(3/2)`);
 expressions.push('1^(1/0)','(-1)^(1/0)','1^(-1/0)','(-1)^(-1/0)','(0/0)^0','(1/0)^0','2^(1/0)','1/(1/0)','0^(-1/0)','(-0)^(-3)','(-0)^3','(-1)^2^3','10\uFEFFin\uFEFFEUR','10\u0085in EUR','10 in EUR\uFEFFat 2023/2/28','10 at ٢٠٢٣/2/28');
 for(const base of ['0','-0','1','-1','2','-2','(1/0)','(-1/0)','(0/0)'])for(const exponent of ['0','-0','1','-1','2','-2','(1/0)','(-1/0)','(0/0)','0.5','-0.5','3','-3'])expressions.push(`(${base})^(${exponent})`);
 // Execute the original helper for ambiguous V8 slash dates, including fields
 // that switch from year/month/day to month/day/year or become invalid.
 for(const year of [...Array(100).keys(),100,1899,1900,2099,2100,2400,9999])
  for(const [month,day] of [[1,1],[2,3],[12,31],[31,12],[0,1],[13,1],[2,0],[2,49],[2,50],[2,99]])
   expressions.push(`1 in EUR at ${String(year).padStart(4,'0')}/${month}/${day}`);
 const parsed=expressions.map(text=>{const q=helper.parseAmountExpression(text);return {text,amount:num(q.amount),filter:q.filter,date:q.historyDate?[q.historyDate.getFullYear(),q.historyDate.getMonth()+1,q.historyDate.getDate()]:null}});
 const numbers=[0,-0,1,-1,1234.5,-1234.56789,1.23445,1.23455,9999.99999,0.00005,-0.00005,1e-7,1e21,1e30,Number.MIN_VALUE,Number.MAX_VALUE,NaN,Infinity,-Infinity,1.005,2.675,999.99995,0.1+0.2];
 const formats=[];for(const code of Object.keys(tables.currencyCode2Name)){for(const value of numbers)formats.push({code,input:String(value),negativeZero:Object.is(value,-0),expected:helper.formatCurrency(value,code)});}
 const enrich=[];for(const base of ['USD','EUR','GBP','JPY'])for(const filter of ['','eur','japan','dollar'])for(const pinned of [undefined,['GBP','EUR','JPY'],['EUR','EUR','ZZZ']]){
  const data=JSON.parse(JSON.stringify(fixture.body)),result=source.enrichExchangeData(data,123.456,base,filter,pinned);enrich.push({base,filter,pinned:pinned||[],rates:data.conversion_rates,expected:[result.conversion_rate_pin_exchanged||[],result.conversion_rate_exchanged].map(a=>a.map(x=>({code:x.code,value:String(x.value)})))});
 }
 const special=[];for(const rates of [{USD:0,EUR:1,JPY:0},{EUR:1,JPY:0},{USD:1,EUR:-0,JPY:-5},{USD:1e-320,EUR:1e308,JPY:0}]){
 const r=source.enrichExchangeData({result:'success',base_code:'USD',conversion_rates:rates},10,'USD','',['GBP','EUR']);special.push({rates,expected:[r.conversion_rate_pin_exchanged,r.conversion_rate_exchanged].map(a=>a.map(x=>({code:x.code,value:String(x.value)})))});
 }
 let tree=await settle();assert.equal(tree.type,'List');assert.equal(walk(tree,'List.Dropdown.Item').length,Object.keys(tables.currencyCode2CountryAndRegion).length);
 tree.props.onSearchTextChange('10');tree=await settle();assert(walk(tree,'List.Item').some(x=>x.props.title==='EUR'),JSON.stringify({toasts,requests,types:tree.children?.map(x=>x?.type)}));
 const first=walk(tree,'Action.CopyToClipboard')[0];copies.push(first.props.content);assert.equal(copies[0],'1500');
 await walk(tree,'Action').find(x=>x.props.title==='Pin It').props.onAction();tree=await settle();assert.equal(JSON.parse(storage.get('pinnedCurrencyCode'))[0],'JPY');
 const dropdown=tree.props.searchBarAccessory;dropdown.props.onChange('EUR');tree=await settle();assert.equal(storage.get('fromCurrencyCode'),'EUR');
 const dateAction=walk(tree,'Action').find(x=>x.props.title==='Set Currency Date');dateAction.props.onAction();assert.equal(navigation.length,1);
 let form=expand(navigation[0]);assert.equal(form.type,'Form');let submit=expand(form.props.actions);const submitNode=walk(submit,'Action.SubmitForm')[0];submitNode.props.onSubmit({historyDate:new FakeDate(2023,1,28)});tree=await settle();assert.equal(navigation.length,0);assert(tree.props.searchText.includes(' at 2023/2/28'));
 const formEmpty=expand(source.SetCurrencyDateForm({amountExpression:'',setSearchText:()=>{throw Error('unexpected empty date update')}}));walk(expand(formEmpty.props.actions),'Action.SubmitForm')[0].props.onSubmit({});
 storage.clear();requests=[];
 const sourceProvider=[];for(const provider of ['exchangerate-api','unirate'])for(const history of [false,true]){
 prefs={provider,api_key:'owned-fixture-key'};fixture.status=200;fixture.body=provider==='unirate'?{base:'USD',rates:{USD:'1',JPY:'150.5',EUR:'.9',ZZZ:'notnumber',GBP:'0x10'}}:{result:'success',base_code:'USD',conversion_rates:{USD:1,EUR:0.9},...(history?{year:2023,month:2,day:28}:{time_next_update_unix:1735776000,time_last_update_unix:1735689600,time_last_update_utc:'Wed, 01 Jan 2025 00:00:00 GMT'})};
 const date=history?new FakeDate(2023,1,28):null;const r=await source.currencyAPI(date,new AbortController().signal);sourceProvider.push({provider,history,status:r.status,body:await r.json()});
 }
 prefs={provider:'unirate',api_key:'owned-fixture-key'};fixture.status=401;fixture.body={message:'owned fixture invalid'};assert.equal((await (await source.currencyAPI(null,new AbortController().signal)).json())['error-type'],'invalid-key');
 fixture.status=200;fixture.body={base:'USD',rates:{USD:1,EUR:0.9}};storage.clear();const before=requests.length;await source.performExchange(10,'USD',new AbortController().signal,'',null,[]);await source.performExchange(20,'USD',new AbortController().signal,'',null,[]);assert.equal(requests.length,before+1);clock+=86400000;await source.performExchange(20,'USD',new AbortController().signal,'',null,[]);assert.equal(requests.length,before+2);
 storage.clear();const hist=new FakeDate(2023,1,28),h=requests.length;await source.performExchange(10,'USD',new AbortController().signal,'',hist,[]);clock+=86400000*366;await source.performExchange(10,'USD',new AbortController().signal,'',hist,[]);assert.equal(requests.length,h+1);
 const evidence={sourceSHA:'8759bec78fa594cc3c56e9c3746bfd31e78d0c69',node:process.version,icu:process.versions.icu,parsed,formats,enrich,special,sourceProvider,requests,counts:{parse:parsed.length,formats:formats.length,enrich:enrich.length,special:special.length,wholeTSXFlows:8,lockedLibraryModules:loaded.size,discardedSourceConsoleCalls:logs},loaded:[...loaded].sort()};
 fs.writeFileSync(root+'/v2/tests/source-oracle.json',JSON.stringify(evidence,null,2)+'\n');
 console.log(JSON.stringify(evidence.counts));
}
main().catch(e=>{console.error(e.stack);process.exitCode=1});

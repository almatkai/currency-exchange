const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const V=path.resolve(__dirname,'..'),React=require(V+'/.qa/renderer/node_modules/react'),TR=require(V+'/.qa/renderer/node_modules/react-test-renderer'),ts=require(V+'/.qa/renderer/node_modules/typescript');
global.IS_REACT_ACT_ENVIRONMENT=true;
const deny=n=>()=>{throw Error('DENIED '+n)};
let count=0;const check=(v,msg)=>{assert(v,msg);count++};
function world(patched){
 let timers=new Map(),timerId=0;const timeout=(fn)=>{timers.set(++timerId,fn);return timerId};const clear=id=>timers.delete(id);
 const wrap=({children,...p})=>React.createElement('span',p,children),primitive=new Proxy({cx:(...x)=>x.filter(Boolean).join(' ')},{get:(o,k)=>o[k]||wrap});
 const win={addEventListener:()=>{},removeEventListener:()=>{},setTimeout:timeout,clearTimeout:clear,tezbar:new Proxy({}, {get:(_,k)=>deny('tezbar.'+String(k))})};
 const sandbox=vm.createContext({console:{log:()=>{},error:()=>{}},window:win,setTimeout:timeout,clearTimeout:clear,HTMLTextAreaElement:class{},URL,URLSearchParams,TextEncoder,TextDecoder});
 const cache=new Map(),base=V+(patched?'/.qa/patched-host':'/integration/fixtures/host');
 const hooks={NavigationContext:React.createContext({}),setRuntimeContext:()=>{},useNavigation:()=>({}),environment:{},Clipboard:{},getPreferenceValues:deny('settings'),open:deny('open'),showHUD:deny('hud'),showToast:deny('toast'),useFetch:deny('fetch'),usePromise:deny('promise'),useCachedPromise:deny('cache')};
 const registry={ActionRegistryContext:React.createContext({})};
 function load(rel){if(cache.has(rel))return cache.get(rel);const file=base+'/'+rel;let source=fs.readFileSync(file,'utf8');
  if(rel.endsWith('list-runtime.tsx'))source+='\nexport { parseListAccessory,parseListRows };';
  if(rel.endsWith('form-runtime.tsx'))source+='\nexport { collectFormItems };';
  const code=ts.transpileModule(source,{fileName:rel,compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022,jsx:ts.JsxEmit.ReactJSX,esModuleInterop:true}}).outputText;
  const mod={exports:{}};cache.set(rel,mod.exports);
  const local=id=>{if(id==='react')return React;if(id==='react/jsx-runtime')return require(V+'/.qa/renderer/node_modules/react/jsx-runtime');
   if(id.includes('ui/primitives'))return primitive;if(id.includes('ui/Markdown'))return {Markdown:wrap};if(id.endsWith('detail-runtime'))return {MetadataItem:wrap,MetadataSidebar:wrap,parseNativeColorWheel:()=>null,NativeColorWheel:wrap,DetailRuntime:wrap};
   if(id.endsWith('grid-runtime'))return {GridRuntime:wrap};if(id.endsWith('action-runtime'))return {Action:wrap,ActionPanel:wrap,ActionPanelOverlay:({actions,onExecute})=>React.createElement('div',{},actions.map(a=>React.createElement('button',{key:a.id,onClick:()=>onExecute(a)},a.title)))};
   if(id.endsWith('action-runtime-registry'))return registry;if(id.endsWith('colorConverter'))return {buildColorConversionResults:()=>[]};if(id.endsWith('./hooks'))return hooks;
   if(id.endsWith('extensionSessionRefresh'))return {startSessionRefresh:()=>()=>{}};
   let target=path.posix.normalize(path.posix.join(path.posix.dirname(rel),id));if(!path.posix.extname(target))target+=fs.existsSync(base+'/'+target+'.tsx')?'.tsx':'/index.tsx';
   if(fs.existsSync(base+'/'+target))return load(target);throw Error('DENIED import '+id);
  };
  new vm.Script('(function(require,module,exports){'+code+'\n})',{filename:rel}).runInContext(sandbox)(local,mod,mod.exports);cache.set(rel,mod.exports);return mod.exports;
 }
 return {load,win,flush:()=>{for(const [id,fn]of timers){timers.delete(id);fn()}}};
}
const ownOptions=JSON.parse(fs.readFileSync(V+'/src/currencies.json','utf8')).map(x=>({type:'List.Dropdown.Item',props:{value:x.code,title:x.code+' - '+x.country}}));
function view(token='a',query='10',from='USD'){return {ok:true,mode:'view',sessionId:'s',extensionId:'com.tezbar.native.currency-exchange',commandName:'index',title:'Currency Exchange',generation:'g',refreshMs:null,root:{type:'List',props:{__hasServerSearch:true,viewToken:token,searchText:query,fromCurrencyCode:from,searchBarPlaceholder:'Input how much United States Dollar',searchBarAccessory:{type:'List.Dropdown',props:{value:from,actionId:'from-'+token},children:ownOptions}},children:[{type:'List.Section',props:{title:'Exchange from $10'},children:[{type:'List.Item',props:{id:'EUR',title:'EUR',subtitle:'€9',icon:{source:'🇪🇺'},actionIds:['copy-'+token]}}]},{type:'List.EmptyView',props:{title:'No matching currencies',description:'Try a currency name'}}]},actions:[{id:'copy-'+token,title:'Copy to Clipboard',kind:'copy'},{id:'from-'+token,title:'From Currency',kind:'action'}]}}
const mockDOM=()=>({focus:()=>{},querySelector:()=>null,scrollIntoView:()=>{}});
async function mount(component,props){let r;await TR.act(async()=>{r=TR.create(React.createElement(component,props),{createNodeMock:mockDOM})});return r}
function input(r){return r.root.findAllByType('input').find(x=>x.props.type==='text')}
function listDiv(r){return r.root.findAllByType('div').find(x=>x.props.onKeyDown)}
function enter(r){listDiv(r).props.onKeyDown({key:'Enter',repeat:false,preventDefault(){},stopPropagation(){}})}
const deferred=()=>{let resolve;const promise=new Promise(r=>resolve=r);return {promise,resolve}};
function textOutput(n){return typeof n==='string'?n:Array.isArray(n)?n.map(textOutput).join(''):n?textOutput(n.children):''}
async function main(){
 for(const patched of [false,true]){
 const w=world(patched),mod=w.load('src/renderer/src/raycast-api/index.tsx'),list=w.load('src/renderer/src/raycast-api/list-runtime.tsx');
 const v=view('a','10','EUR');let calls=[],search=[],draft=0;const r=await mount(mod.ExtensionRuntimeSurface,{...v,onBack:()=>{},onLoadMore:()=>{},onDraftChanged:()=>draft++,onSearchTextChanged:s=>search.push(s),onInvokeAction:(...a)=>calls.push(a)});
 const selects=r.root.findAllByType('select');check(patched?selects.length===1:selects.length===0,'large dropdown before/after');
 if(patched)check(selects[0].props.value==='EUR','controlled currency');
 await TR.act(async()=>input(r).props.onChange({target:{value:'20'}}));await TR.act(async()=>enter(r));
 check(patched?calls.length===0:calls.length===1,'pending draft must block original copy');if(patched){check(search[0]==='20','native search immediate');check(draft===1);}
 await TR.act(async()=>r.update(React.createElement(mod.ExtensionRuntimeSurface,{...view('b','20','EUR'),onBack:()=>{},onLoadMore:()=>{},onDraftChanged:()=>{},onSearchTextChanged:()=>{},onInvokeAction:(...a)=>calls.push(a)})));
 await TR.act(async()=>enter(r));if(patched){check(calls.length===1);check(calls[0][1].__tezbarViewToken==='b');check(calls[0][1].__tezbarSearchText==='20');check(calls[0][1].__tezbarFromCurrency==='EUR');}
 const parsed=list.parseListRows({type:'List',children:[{type:'List.Item',props:{title:'<img src=x onerror=owned()>',subtitle:'<script>owned()</script>',icon:{source:'🇺🇳'},actionIds:['x']}}]});check(parsed[0].title==='<img src=x onerror=owned()>');
 await TR.act(async()=>r.unmount());
 }
 const w=world(true),mod=w.load('src/renderer/src/raycast-api/index.tsx'),form=w.load('src/renderer/src/raycast-api/form-runtime.tsx');let values;
 const f={...view(),root:{type:'Form',props:{viewToken:'form',searchText:'10 in eur',fromCurrencyCode:'USD',backActionId:'cancel'},children:[{type:'Form.DatePicker',props:{id:'historyDate',title:'Currecy History Date',value:'2023-02-28'}}]},actions:[{id:'submit',title:'Set to Date',kind:'submit-form'},{id:'cancel',title:'Cancel',kind:'pop'}]};
 const pending=deferred();const r=await mount(mod.ExtensionRuntimeSurface,{...f,onBack:()=>{},onDraftChanged:()=>{},onLoadMore:()=>{},onSearchTextChanged:()=>{},onInvokeAction:(id,v)=>{values=v;return pending.promise}});
 let date=r.root.findByType('input');check(date.props.type==='date');check(date.props.value==='2023-02-28');await TR.act(async()=>date.props.onChange({target:{value:'2024-01-15'}}));await TR.act(async()=>r.root.findByType('form').props.onSubmit({preventDefault(){}}));check(values.historyDate==='2024-01-15');check(values.__tezbarViewToken==='form');check(r.root.findByType('fieldset').props.disabled===true,'pending submit freezes its authorized field snapshot');
 await TR.act(async()=>pending.resolve());check(r.root.findByType('fieldset').props.disabled===false);await TR.act(async()=>r.unmount());
 check(form.collectFormItems(f.root)[0].field.type==='Form.DatePicker');
 const malicious=view('safe');malicious.root.children[0].children[0].props.title='<img src=x onerror=owned()>';malicious.root.children[0].children[0].props.subtitle='<script>owned()</script>';const plain=await mount(mod.ExtensionRuntimeSurface,{...malicious,onBack:()=>{},onLoadMore:()=>{},onSearchTextChanged:()=>{},onInvokeAction:()=>{}});check(plain.root.findAllByType('script').length===0);check(plain.root.findAll(x=>x.props.dangerouslySetInnerHTML).length===0);check(textOutput(plain.toJSON()).includes('<script>owned()</script>'));await TR.act(async()=>plain.unmount());
 const empty=view('empty','');empty.root.children=empty.root.children.slice(1);const png=fs.readFileSync(V+'/assets/start.png');empty.root.children[0].props.icon={source:'data:image/png;base64,'+png.toString('base64')};const emptyRendered=await mount(mod.ExtensionRuntimeSurface,{...empty,onBack:()=>{},onLoadMore:()=>{},onSearchTextChanged:()=>{},onInvokeAction:()=>{}});const image=emptyRendered.root.findByType('img');check(crypto.createHash('sha256').update(Buffer.from(image.props.src.split(',')[1],'base64')).digest('hex')==='d56bdab6a03e3a67b5e158c24c904a558dbea659dd30e82fd7c2ec69dfb758ad');check(textOutput(emptyRendered.toJSON()).includes('No matching currencies'));await TR.act(async()=>emptyRendered.unmount());
 // Actual pinned RuntimeView and Surface/List: late A/B/A searches and old action responses.
 for(const patched of [false,true]){
 const w=world(patched),Runtime=w.load('src/renderer/ExtensionRuntimeView.tsx').default;let pendingActions=[],pendingSearch=[];
 w.win.tezbar={extensionInvokeAction:()=>{const d=deferred();pendingActions.push(d);return d.promise},extensionSearchTextChanged:q=>{const d=deferred();pendingSearch.push({q,d});return d.promise},extensionDisposeSession:async()=>true};
 const r=await mount(Runtime,{initial:view(),onBack:()=>{}});await TR.act(async()=>enter(r));await TR.act(async()=>input(r).props.onChange({target:{value:'20'}}));if(!patched)await TR.act(async()=>w.flush());
 await TR.act(async()=>pendingActions[0].resolve(view('late-action','OLD')));
 check(patched?input(r).props.value==='20':input(r).props.value==='OLD','action response draft fence before/after');
 if(patched){await TR.act(async()=>input(r).props.onChange({target:{value:'10'}}));check(pendingSearch.length===2);await TR.act(async()=>pendingSearch[0].d.resolve(view('older','20')));check(input(r).props.value==='10');await TR.act(async()=>pendingSearch[1].d.resolve(view('newest','10')));check(input(r).props.value==='10');}
 await TR.act(async()=>r.unmount());
 }
 // The actual selected producer's builtin Copy has no window.hide/visibility effect.
 const producer=fs.readFileSync(V+'/integration/fixtures/host/src/main/extension-runner.ts','utf8');let start=producer.indexOf("    if (kind === 'copy')",producer.indexOf('function registerAction(')),end=producer.indexOf("    if (kind === 'open')",start);const block=producer.slice(start,end);let copied=[],pasted=0;const copy=vm.runInNewContext('(async()=>{'+block+'})',{kind:'copy',props:{content:'123.456'},session:{effectMode:'execute'},typeName:'Action.CopyToClipboard',copyToSystemClipboard:async(_,v)=>copied.push(v),pasteClipboardContent:async()=>pasted++});await copy();check(copied[0]==='123.456'&&pasted===0);check(!/hide|visibility/i.test(block));
 console.log(JSON.stringify({rendererAssertions:count,react:React.version,actualPinnedFiles:4,unpatchedFailuresReproduced:4}));fs.writeFileSync(V+'/.qa/renderer-results.json',JSON.stringify({assertions:count,react:React.version},null,2));
}
main().catch(e=>{console.error(e.stack);process.exitCode=1});

"""Real extracted executable, LF JSONL, owned data only, OS-denied network."""
from pathlib import Path
import hashlib,json,os,select,subprocess,sys,time,uuid
V=Path(__file__).resolve().parent.parent
binary=Path(sys.argv[1]) if len(sys.argv)>1 else V/'.qa/extracted/bin/currency-exchange'
root=V/'.qa/process';root.mkdir(exist_ok=True);key='owned-fixture-key'
profile='(version 1)(allow default)(deny network*)(deny file-read* (subpath "/Users"))(allow file-read* (subpath '+json.dumps(str(V.parent)) +'))(deny file-write*)(allow file-write* (subpath '+json.dumps(str(V.parent))+'))'
env={'HOME':str(V/'.qa/home'),'TMPDIR':str(V/'.qa/tmp'),'PATH':'/usr/bin:/bin','TZ':'Asia/Almaty'}
p=subprocess.Popen(['/usr/bin/sandbox-exec','-p',profile,str(binary)],cwd=V,env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,bufsize=0)
counts={'realProcess':True,'networkDeniedByOS':True,'commands':1,'assertions':0}
def check(v):assert v;counts['assertions']+=1
def send(v):p.stdin.write((json.dumps(v,ensure_ascii=False)+'\n').encode());p.stdin.flush()
def recv():
 line=bytearray();deadline=time.monotonic()+5
 while not line.endswith(b'\n'):
  assert time.monotonic()<deadline,'process timeout'
  ready=select.select([p.stdout],[],[],max(0,deadline-time.monotonic()))[0];assert ready,'process timeout'
  b=os.read(p.stdout.fileno(),1);assert b,'process EOF';line.extend(b);assert len(line)<16*1024*1024
 return json.loads(line)
def request(method,params):
 id='h:'+str(uuid.uuid4());send({'type':'request','id':id,'method':method,'params':params});return id
def response(id):
 while True:
  v=recv()
  if v.get('id')==id:return v
  assert v['type'] in ['cancel','event'],v['type']
def call(op,payload):return request('session.call',{'sessionId':'s','operation':op,'payload':payload})
def action(view,title,fields=None):
 fields=dict(fields or {});pr=view['root']['props'];fields.update(__tezbarViewToken=pr['viewToken'],__tezbarSearchText=pr['searchText'],__tezbarFromCurrency=pr['fromCurrencyCode']);return {'actionId':next(a['id'] for a in view['actions'] if a['title']==title),'formValues':fields}
def rows(view):return [row for section in view['root']['children'] if section['type']=='List.Section' for row in section['children']]
check(recv()['id']=='com.tezbar.native.currency-exchange')
check(response(request('initialize',{'protocol':1,'extensionId':'com.tezbar.native.currency-exchange','platform':'macOS','packageDir':str(binary.parent.parent),'dataDir':str(root/'data'),'tempDir':str(root/'temp'),'grantedCapabilities':['clipboard.write'],'migrationPaths':{}}))['result']['ready'])
store=root/'data/currency-exchange/storage.json';store.parent.mkdir(parents=True,exist_ok=True)
identity='exchangerate-api:'+hashlib.sha256(key.encode()).hexdigest()+':current'
store.write_text(json.dumps({'unknown':'keep','cache':{identity:{'data':{'rates':{'USD':1,'JPY':150,'EUR':0.9,'GBP':0.8},'next':4102444800,'last':1735689600,'utc':'Wed, 01 Jan 2025 00:00:00 GMT','date':None}}}}))
v=response(request('session.open',{'sessionId':'s','commandName':'index','preferences':{'provider':'exchangerate-api','api_key':key},'argumentValues':{},'searchText':'10'}))['result'];check(rows(v)[0]['props']['title']=='JPY')
old=action(v,'Copy to Clipboard');q=call('extension:invoke-action',old);reverse=recv();check(reverse['method']=='host.clipboard.write');check(reverse['params']=={'text':'1500'});send({'type':'response','id':reverse['id'],'result':None});v=response(q)['result'];check('unknown' in v['message'])
q=call('extension:invoke-action',action(v,'Retry exact clipboard write'));retry=recv();check(retry['params']==reverse['params']);send({'type':'response','id':retry['id'],'result':{'ok':True}});v=response(q)['result'];check(v['message']=='Copied to Clipboard');check(action(v,'Copy to Clipboard')['actionId']!=old['actionId']);check(response(call('extension:invoke-action',old))['result']['mode']=='unchanged')
v=response(call('extension:invoke-action',action(v,'Pin It')))['result'];check(rows(v)[0]['props']['id']=='pin:JPY')
v=response(call('extension:invoke-action',action(v,'From Currency',{'value':'EUR'})))['result'];check(v['root']['props']['fromCurrencyCode']=='EUR');check(v['root']['props']['searchText']=='10')
v=response(call('extension:invoke-action',action(v,'Set Currency Date')))['result'];check(v['root']['type']=='Form');v=response(call('extension:invoke-action',action(v,'Set to Date',{'historyDate':''})))['result'];check(v['root']['type']=='List');check(v['root']['props']['searchText']=='10')
old=action(v,'Copy to Clipboard');v=response(call('extension:search-text-changed',{'searchText':'20 in japan'}))['result'];check(rows(v)[0]['props']['title']=='JPY');check(response(call('extension:invoke-action',old))['result']['mode']=='unchanged');check(response(call('extension:load-more',{}))['result']['mode']=='unchanged')
v=response(call('extension:refresh-session',{}))['result'];check(v['root']['props']['searchText']=='20 in japan');check(json.loads(store.read_text())['unknown']=='keep')
check(response(request('session.dispose',{'sessionId':'s'}))['result']['mode']=='unchanged');p.stdin.close();check(p.wait(timeout=4)==0);stderr=p.stderr.read();check(key.encode() not in stderr);check(not stderr)
(V/'.qa/process-results.json').write_text(json.dumps(counts,indent=2)+'\n');print(json.dumps(counts))

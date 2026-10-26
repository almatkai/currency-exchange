"""Build a reviewable, UNAPPLIED patch from exact pinned host fixtures."""
from pathlib import Path
import difflib,hashlib,json,shutil
V=Path(__file__).resolve().parent.parent
F=V/'integration/fixtures/host'; src=V/'.qa/contracts/host'; out=V/'.qa/patched-host'
paths=['src/renderer/src/raycast-api/list-runtime.tsx','src/renderer/src/raycast-api/form-runtime.tsx','src/renderer/src/raycast-api/index.tsx','src/renderer/ExtensionRuntimeView.tsx','src-tauri/src/native_core/extensions/native.rs','src-tauri/src/native_core/extensions/registry.rs','src-tauri/src/native_core/extensions/v2_runtime.rs','schemas/extension-v3.schema.json']
for p in paths:
 q=F/p;q.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(src/p,q)
# Producer and clipboard boundary: exact whole files in fixture-only inputs.
for p in ['src/main/extension-runner.ts','src-tauri/src/native_core/launcher/clipboard.rs','src-tauri/src/native_core/extensions/process_host.rs','src/shared/extensionRuntime.ts','LICENSE']:
 q=F/p;q.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(src/p,q)
index={p.relative_to(F).as_posix():hashlib.sha256(p.read_bytes()).hexdigest() for p in F.rglob('*') if p.is_file()};(V/'integration/host-fixtures.json').write_text(json.dumps({'pin':'ea100cee3f69711de12485ba061950c5fc65258f','files':index},indent=2)+'\n')
modified={}
def read(p):return modified.get(p,(F/p).read_text())
def put(p,s):modified[p]=s
def replace(p,a,b):
 s=read(p);assert s.count(a)==1,(p,a[:80],s.count(a));put(p,s.replace(a,b))
L=paths[0];S=paths[2];R=paths[3];N=paths[4];G=paths[5]
# Surface owns immediate draft invalidation and tokens for every action path, including overlay/Back.
replace(S,'  onLoadMore: () => Promise<void> | void','  onDraftChanged?: () => void\n  onLoadMore: () => Promise<void> | void')
replace(S,'    onInvokeAction,\n  } = props','    onInvokeAction: invokeAction,\n    onDraftChanged,\n  } = props\n  const draftBlocked = useRef(false)\n  const formBusyRef = useRef(false)\n  const [formBusy, setFormBusy] = useState(false)\n  const nativeToken = typeof root.props?.viewToken === \'string\' ? root.props.viewToken : null\n  useEffect(() => { draftBlocked.current = false }, [nativeToken])\n  const onInvokeAction = useCallback(async (id: string, values?: Record<string, unknown>) => {\n    if (nativeToken && (draftBlocked.current || formBusyRef.current)) return\n    const submitting = Boolean(nativeToken && root.type === \'Form\' && actions.some(a => a.id === id && a.kind === \'submit-form\'))\n    if (submitting) { formBusyRef.current = true; setFormBusy(true) }\n    try {\n      const pending = invokeAction(id, nativeToken ? { ...values,\n        __tezbarViewToken: nativeToken, __tezbarSearchText: root.props?.searchText,\n        __tezbarFromCurrency: root.props?.fromCurrencyCode } : values)\n      if (nativeToken && id === (root.props?.searchBarAccessory as ExtensionRuntimeNode | undefined)?.props?.actionId) draftBlocked.current = true\n      await pending\n    } finally { if (submitting) { formBusyRef.current = false; setFormBusy(false) } }\n  }, [invokeAction, nativeToken, root, actions])\n  const onDraft = useCallback(() => { draftBlocked.current = true; onDraftChanged?.() }, [onDraftChanged])')
replace(S,'              onSearchTextChanged={onSearchTextChanged}\n              onLoadMore={onLoadMore}','              onSearchTextChanged={onSearchTextChanged}\n              onDraftChanged={onDraft}\n              onLoadMore={onLoadMore}')
# Actual Form already supports DatePicker and complete submit snapshots. Fence pending field edits, no invented Date API.
replace(S,'              onOpenActions={openActions}\n              actionNotice={actionNotice}','              onOpenActions={openActions}\n              onDraftChanged={onDraftChanged}\n              disabled={formBusy}\n              actionNotice={actionNotice}')
FPath=paths[1]
replace(FPath,'  onChangeField: (actionId: string, value: unknown) => Promise<void> | void','  onDraftChanged?: () => void\n  onChangeField: (actionId: string, value: unknown) => Promise<void> | void')
replace(FPath,'onSubmitForm, onChangeField, onInvokeAction','onSubmitForm, onChangeField, onDraftChanged, onInvokeAction')
replace(FPath,'    setValues((previous) => ({ ...previous, [field.id]: value }))','    onDraftChanged?.()\n    setValues((previous) => ({ ...previous, [field.id]: value }))')
replace(FPath,'  root: ExtensionRuntimeNode\n  title: string','  root: ExtensionRuntimeNode\n  disabled?: boolean\n  title: string')
replace(FPath,'{ root, title, onBack,','{ root, disabled = false, title, onBack,')
replace(FPath,'    onDraftChanged?.()\n    setValues','    if (disabled) return\n    onDraftChanged?.()\n    setValues')
s=read(FPath).replace('onSubmitForm(values)','!disabled && onSubmitForm(values)')
s=s.replace('<div className="mx-auto max-w-[900px] space-y-4">','<fieldset disabled={disabled} className="mx-auto max-w-[900px] space-y-4">')
s=s.replace('        </div>\n      </div>\n\n      {actionNotice','        </fieldset>\n      </div>\n\n      {actionNotice').replace('            type="submit"','            disabled={disabled}\n            type="submit"');put(FPath,s)
# Controlled dropdown, actual native large <select>, and immediate native searches. Legacy debounce retained.
replace(L,'  actionId?: string\n  options:','  actionId?: string\n  value?: string\n  options:')
replace(L,"    actionId: typeof node.props?.actionId === 'string' ? node.props.actionId : undefined,","    value: typeof node.props?.value === 'string' ? node.props.value : undefined,\n    actionId: typeof node.props?.actionId === 'string' ? node.props.actionId : undefined,")
replace(L,'  onSearchTextChanged: (searchText: string) => Promise<void> | void','  onDraftChanged?: () => void\n  onSearchTextChanged: (searchText: string) => Promise<void> | void')
replace(L,'  onSearchTextChanged,','  onSearchTextChanged,\n  onDraftChanged,')
replace(L,"    searchAccessory?.options[0]?.value ?? '',","    searchAccessory?.value ?? searchAccessory?.options[0]?.value ?? '',")
replace(L,"      if (!searchAccessory || searchAccessory.options.length === 0) return ''","      if (!searchAccessory || searchAccessory.options.length === 0) return ''\n      if (searchAccessory.value !== undefined) return searchAccessory.value")
replace(L,"    if (query === lastSentQuery.current) return","    if (root.props?.viewToken) return\n    if (query === lastSentQuery.current) return")
replace(L,'  }, [query, onSearchTextChanged])','  }, [query, onSearchTextChanged, root.props?.viewToken])')
replace(L,'                  setQuery(event.target.value)\n                  setSelected(0)','                  onDraftChanged?.()\n                  setQuery(event.target.value)\n                  setSelected(0)\n                  if (root.props?.viewToken) void onSearchTextChanged(event.target.value)')
replace(L,'            {searchAccessory && searchAccessory.options.length > 0 ? (','            {searchAccessory && searchAccessory.options.length > 20 ? (\n              <select aria-label="From Currency" value={accessoryValue} onChange={(event) => {\n                if (searchAccessory.actionId) onRunPrimaryAction(searchAccessory.actionId, { value: event.target.value })\n              }}>\n                {searchAccessory.options.map(option => <option key={option.value} value={option.value}>{option.title}</option>)}\n              </select>\n            ) : searchAccessory && searchAccessory.options.length > 0 ? (')
# Server-provided empty feedback is authoritative on a server-search List.
replace(L,"  const emptyTitle = isColorConversionSurface","  const emptyTitle = hasServerSearch && emptyView ? emptyView.title : isColorConversionSurface")
replace(L,"  const emptyDescription = isColorConversionSurface","  const emptyDescription = hasServerSearch && emptyView ? emptyView.description : isColorConversionSurface")
replace(L,"      title: typeof candidate.props?.title === 'string' ? candidate.props.title : 'No results',", "      icon: parseRowIcon(candidate.props?.icon),\n      title: typeof candidate.props?.title === 'string' ? candidate.props.title : 'No results',")
replace(L,'              <p className="text-[13px] text-ink-2">','              {emptyView?.icon ? <div className="mb-3 flex justify-center"><RowIcon row={{ id: \'empty\', title: emptyTitle, subtitle: \'\', icon: emptyView.icon }} /></div> : null}\n              <p className="text-[13px] text-ink-2">')
# Runtime fences ALL response families against draft changes (including A/B/A and Form edits).
replace(R,'            onSearchTextChanged={handleSearchTextChanged}','            onDraftChanged={() => { searchRequestSeq.current += 1 }}\n            onSearchTextChanged={handleSearchTextChanged}')
replace(R,'              const result = await window.tezbar.extensionInvokeAction({','              const actionEpoch = searchRequestSeq.current\n              const result = await window.tezbar.extensionInvokeAction({')
replace(R,'              if (currentSession.current !== sessionOwner) return\n              if (!result.ok)', '              if (currentSession.current !== sessionOwner || actionEpoch !== searchRequestSeq.current) return\n              if (result.ok && result.mode === \'unchanged\') return\n              if (!result.ok)')
# Refresh must not overwrite a newer draft; no timers are added.
replace(R,'          const result = await window.tezbar.extensionRefreshSession({ sessionId: state.sessionId })','          const refreshEpoch = searchRequestSeq.current\n          const result = await window.tezbar.extensionRefreshSession({ sessionId: state.sessionId })')
replace(R,"          if (cancelled || (result.ok && result.mode === 'unchanged')) return","          if (cancelled || refreshEpoch !== searchRequestSeq.current || (result.ok && result.mode === 'unchanged')) return")
replace(R,'    const result = await window.tezbar.extensionLoadMore({ sessionId: state.sessionId })','    const loadEpoch = searchRequestSeq.current\n    const result = await window.tezbar.extensionLoadMore({ sessionId: state.sessionId })')
replace(R,'    if (currentSession.current !== sessionOwner) return\n    if (result.ok', '    if (currentSession.current !== sessionOwner || loadEpoch !== searchRequestSeq.current) return\n    if (result.ok')
# Canonical damaged directories keep their existing error path. Only verified, exact,
# unambiguous declared replacement aliases with a covered platform command enter Rust.
helper='''pub(super) fn verified_native_alias(ctx: &CoreContext, requested: &str, command: &str) -> Option<String> {
    let mut matches = collect(ctx).into_iter().filter(|entry| {
        entry.id.starts_with("com.tezbar.native.") &&
        entry.manifest["healthError"].is_null() &&
        entry.manifest["replaces"].as_array().is_some_and(|a| a.iter().any(|id| id.as_str() == Some(requested))) &&
        super::process_install::resolve(ctx, &entry.id).is_ok() &&
        installed_command(ctx, &entry.id, command).is_some()
    }).map(|e| e.id);
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

'''
replace(G,'fn platform() -> &\'static str {',helper+'fn platform() -> &\'static str {')
replace(N,'if requested.starts_with("com.tezbar.native.") || v2_runtime::slug(requested).is_some()','if requested.starts_with("com.tezbar.native.") || registry::verified_native_alias(ctx, requested, body["commandName"].as_str().unwrap_or("")).is_some()')
replace(N,'v2_runtime::canonical_id(v2_runtime::slug(requested).ok_or("Unknown extension")?)','registry::verified_native_alias(ctx, requested, command).ok_or("Unknown or uncovered extension alias")?')
# Registry resource paths must use the verified active version, separate from canonical preferences.
resource='''fn resource_root(extension: &Installed) -> Option<PathBuf> {
    if !extension.id.starts_with("com.tezbar.native.") { return Some(extension.path.clone()); }
    let ctx = CoreContext::new(crate::native_core::CorePaths::new(extension.data_dir.clone()));
    super::process_install::resolve(&ctx, &extension.id).ok().map(|p| p.dir)
}
'''
# Do not invent a CoreContext constructor. Keep resource dir explicitly in Installed instead.
replace(G,'    manifest: Value,\n    data_dir: PathBuf,','    manifest: Value,\n    data_dir: PathBuf,\n    // Active verified package resources; canonical path above owns preferences.\n    resource_dir: PathBuf,')
replace(G,'            found.push(Installed {','            let resource_dir = if native { super::process_install::resolve(ctx, &id).ok().map(|p| p.dir).unwrap_or_else(|| path.clone()) } else { path.clone() };\n            found.push(Installed {')
replace(G,'                data_dir: ctx.paths.data_dir.clone(),','                data_dir: ctx.paths.data_dir.clone(),\n                resource_dir,')
# Other constructor in retain_preferences, and tests through helpers use collect; check all constructors explicitly.
replace(G,'        path,\n        manifest: Value::Null,','        resource_dir: path.clone(),\n        path,\n        manifest: Value::Null,')
s=read(G);s=s.replace('extension.path.join(relative)','extension.resource_dir.join(relative)').replace('extension.path.join("assets").join(relative)','extension.resource_dir.join("assets").join(relative)');s=s.replace('\"extensionPath\":extension.path,','\"extensionPath\":extension.resource_dir,');put(G,s)
# Types cover unchanged already returned by real SDK/native dispatcher.
T='src/shared/extensionRuntime.ts';s=read(T);needle='export type ExtensionInvokeActionResult =\n';s=s.replace(needle,needle+"  | { ok: true; mode: 'unchanged' }\n");put(T,s)
patch=[]
for p,s in modified.items():
 q=out/p;q.parent.mkdir(parents=True,exist_ok=True);q.write_text(s)
 patch.extend(difflib.unified_diff((F/p).read_text().splitlines(True),s.splitlines(True),fromfile='a/'+p,tofile='b/'+p))
for p in paths:
 if p not in modified:q=out/p;q.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(F/p,q)
(V/'integration/host-prerequisites.patch').write_text(''.join(patch))
print('unapplied patch:',len(modified),'files')
# Extract complete selected functions, verbatim, for a standalone native boundary
# test with mocked package discovery/verification and OS clipboard. This does not build Tezbar.
def function(text,name):
 start=text.index('fn '+name+'(');start=text.rfind('\n',0,start)+1
 # Braces in these selected funcs are balanced strings too; assertions compile extracted output.
 brace=text.index('{',start);depth=1;i=brace+1
 while depth:
  if text[i]=='{':depth+=1
  elif text[i]=='}':depth-=1
  i+=1
 return text[start:i]+'\n'
extracted=[]
for label,source in [('before',(F/G).read_text()),('after',read(G))]:
 body=''.join(function(source,n) for n in ['slug','icon_path','preference_metadata','preference_file','retained_preferences_path','preferences_path','installed_command'])
 if label=='after':body+=function(source,'verified_native_alias')
 native=(F/N).read_text() if label=='before' else read(N)
 a=native.index('        let canonical = if requested.starts_with(');b=native.index('        let slot = self.slot',a)
 route=native[a:b].strip().replace('let canonical =','let canonical =')+'\nOk(canonical)'
 body+='\npub(super) fn route(ctx: &CoreContext,requested: &str, command: &str)->Result<String,String>{\nlet _ = (ctx,command);\n'+route+'\n}\n'
 body+='\npub(super) fn test_icon(e:&Installed)->Option<String>{icon_path(e)}\npub(super) fn test_preferences(e:&Installed)->Result<PathBuf,String>{preferences_path(e)}\n'
 extracted.append('#[allow(clippy::all)]\nmod '+label+' {\nuse super::*;\n'+('use super::after as registry;\n' if label=='after' else '')+body+'\n}\n')
(V/'integration/native-extracted.rs').write_text('// Exact complete functions / canonical-selection block from host fixtures.\n'+''.join(extracted))

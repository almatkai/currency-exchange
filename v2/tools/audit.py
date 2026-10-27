from pathlib import Path
import hashlib,json,subprocess
V=Path(__file__).resolve().parent.parent;root=V.parent
assert subprocess.check_output(['git','rev-parse','--show-toplevel'],cwd=root,text=True).strip()==str(root)
assert subprocess.check_output(['git','branch','--show-current'],cwd=root,text=True).strip()=='feat/native-v2-rust'
assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()=='8759bec78fa594cc3c56e9c3746bfd31e78d0c69'
assert not subprocess.check_output(['git','diff','--cached','--name-only'],cwd=root)
source=json.loads((V/'tests/v1-hashes.json').read_text())
for name,sha in source.items():
 if name=='README.md':
  original=subprocess.check_output(['git','show','HEAD:README.md'],cwd=root)
  assert (root/name).read_bytes()==original+b'\nNative Rust V2 development port: [v2/README.md](v2/README.md).\n'
 else:assert hashlib.sha256((root/name).read_bytes()).hexdigest()==sha,name
assert (V/'assets/currency.png').read_bytes()==(root/'assets/currency.png').read_bytes()
assert (V/'assets/start.png').read_bytes()==(root/'assets/start.png').read_bytes()
for path in subprocess.check_output(['git','diff','--name-only'],cwd=root,text=True).splitlines():assert path=='README.md' or path.startswith('v2/'),path
fixture=json.loads((V/'integration/host-fixtures.json').read_text())
for p,d in fixture['files'].items():assert hashlib.sha256((V/'integration/fixtures/host'/p).read_bytes()).hexdigest()==d,p
files=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard','-z'],cwd=root).decode().split('\0')
lines=[]
for name in sorted(set(filter(None,files))):
 if name=='v2/INDEX.sha256':continue
 p=root/name;assert p.is_file() and not p.is_symlink(),name
 assert not any(x in name.split('/') for x in ['target','node_modules','.qa']),name
 lines.append(hashlib.sha256(p.read_bytes()).hexdigest()+'  '+name)
(V/'INDEX.sha256').write_text('\n'.join(lines)+'\n')
result={'canonicalRoot':str(root),'branch':'feat/native-v2-rust','HEAD':'8759bec78fa594cc3c56e9c3746bfd31e78d0c69','stagedFiles':0,'originalFilesAudited':len(source),'readmeOnlyDiscoveryLink':True,'inventoryFiles':len(lines),'hostFixtureFiles':len(fixture['files'])}
(V/'.qa/final-audit.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))

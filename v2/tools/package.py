from pathlib import Path
import hashlib,json,shutil,zipfile,sys,stat
V=Path(__file__).resolve().parent.parent
package=V/'.qa/package';package.mkdir(exist_ok=True)
files={'bin/currency-exchange':V/'target/release/currency-exchange','resources/currency.png':V/'assets/currency.png','resources/start.png':V/'assets/start.png','LICENSE':V/'LICENSE','NOTICE':V/'NOTICE','PROVENANCE.json':V/'PROVENANCE.json','DEPENDENCIES.json':V/'DEPENDENCIES.json'}
for p in (V/'licenses').rglob('*'):
 if p.is_file():files['licenses/'+p.relative_to(V/'licenses').as_posix()]=p
manifest=json.loads((V/'extension.json').read_text());manifest['files']={}
for name,path in sorted(files.items()):
 data=path.read_bytes();q=package/name;q.parent.mkdir(parents=True,exist_ok=True);q.write_bytes(data);manifest['files'][name]=hashlib.sha256(data).hexdigest()
 if name.startswith('bin/'):q.chmod(0o755)
(package/'extension.json').write_text(json.dumps(manifest,indent=2)+'\n')
archive=V/'.qa/currency-exchange-aarch64-apple-darwin-dev.zip'
with zipfile.ZipFile(archive,'w',compression=zipfile.ZIP_DEFLATED) as z:
 for p in sorted(package.rglob('*')):
  if p.is_file():z.write(p,p.relative_to(package).as_posix())
extracted=V/'.qa/extracted';extracted.mkdir(exist_ok=True)
with zipfile.ZipFile(archive) as z:
 names=z.namelist();assert len(names)==len(set(names));assert set(names)==set(manifest['files'])|{'extension.json'}
 for name in names:assert not name.startswith('/') and '..' not in Path(name).parts and '\\' not in name
 z.extractall(extracted)
for name,digest in manifest['files'].items():assert hashlib.sha256((extracted/name).read_bytes()).hexdigest()==digest
(extracted/'bin/currency-exchange').chmod(0o755)
result={'archive':archive.name,'archiveSHA256':hashlib.sha256(archive.read_bytes()).hexdigest(),'executableSHA256':manifest['files']['bin/currency-exchange'],'files':len(manifest['files']),'artifacts':manifest['artifacts']}
(V/'.qa/package-results.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))

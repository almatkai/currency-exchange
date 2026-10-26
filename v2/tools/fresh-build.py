from pathlib import Path
import hashlib,json,os,shutil,subprocess,sys,re
V=Path(__file__).resolve().parent.parent;tag=sys.argv[1] if len(sys.argv)>1 else 'fresh-final'
assert re.fullmatch(r'[a-z0-9-]{1,64}',tag)
copy=V/('.qa/'+tag+'-source')
for name in ['source','cargo-home','target','home']:assert not (V/('.qa/'+tag+'-'+name)).exists(),'Use another tag; existing data is never removed.'
copy.mkdir()
paths=[]
for p in V.rglob('*'):
 rel=p.relative_to(V)
 if any(x in rel.parts for x in ['target','.qa','node_modules']) or p.suffix=='.zip' or rel.as_posix()=='INDEX.sha256':continue
 if p.is_file():
  q=copy/rel;q.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(p,q);paths.append(rel)
hashfile=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
source={str(p):hashfile(V/p) for p in paths}
env=dict(os.environ);env.update(CARGO_HOME=str(V/('.qa/'+tag+'-cargo-home')),CARGO_TARGET_DIR=str(V/('.qa/'+tag+'-target')),HOME=str(V/('.qa/'+tag+'-home')),TMPDIR=str(V/'.qa/tmp'),RUSTUP_HOME='/Users/almat/.rustup')
for p in ['CARGO_HOME','CARGO_TARGET_DIR','HOME']:Path(env[p]).mkdir(exist_ok=True)
with (V/'.qa/fresh-build.log').open('w') as log:subprocess.run(['cargo','build','--locked','--release'],cwd=copy,env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
assert all(hashfile(V/p)==source[str(p)]==hashfile(copy/p) for p in paths)
meta=json.loads(subprocess.check_output(['cargo','metadata','--locked','--format-version','1'],cwd=copy,env=env,stderr=subprocess.DEVNULL))
sdk=next(p for p in meta['packages'] if p['name']=='tezbar-extension-sdk');assert sdk['source']=='git+https://github.com/almatkai/tezbar-extension-sdk-rust.git?rev=e955911f0bf5b558f9d3eb5183df0adc95c22af2#e955911f0bf5b558f9d3eb5183df0adc95c22af2';assert sdk['manifest_path'].startswith(env['CARGO_HOME']+'/git/checkouts/')
result={'freshCargoHome':True,'immutableHTTPSSDK':sdk['source'],'sourceFilesCompared':len(paths),'productSourceDigest':hashlib.sha256(json.dumps({str(p):source[str(p)] for p in paths if str(p).startswith('src/') or str(p)in ['Cargo.toml','Cargo.lock']},sort_keys=True).encode()).hexdigest(),'releaseExecutableSHA256':hashfile(Path(env['CARGO_TARGET_DIR'])/'release/currency-exchange')}
(V/'.qa/fresh-results.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))

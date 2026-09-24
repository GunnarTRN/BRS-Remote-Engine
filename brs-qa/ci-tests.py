"""Test the patched build source without touching Windows policy or invoking SAS."""
from pathlib import Path
import subprocess, re, json, hashlib, shutil
root=Path(__file__).resolve().parent
output=root/'test-output'
output.mkdir(exist_ok=True)
harness=(root/'harness.rs').read_text()
source=(root.parent/'src/platform/windows.rs').read_text()
start=source.index('pub fn send_sas() {')
end=source.index('\nlazy_static::lazy_static!',start)
actual=source[start:end].strip()
assert actual == (root/'send_sas.rs').read_text().strip(), 'Compiled source differs from tested patch'
results={}
for name, body in [('before',(root/'send_sas-before.rs').read_text()),('patched',actual)]:
    digest=hashlib.sha256(body.encode()).hexdigest()
    body,n=re.subn(r'    #\[link\(name = "sas"\)\]\n    extern "system" \{\n        pub fn SendSAS\(AsUser: BOOL\);\n    \}\n','',body)
    assert n==1
    generated=output/(name+'_test.rs')
    generated.write_text(harness.replace('// FUNCTION_UNDER_TEST',body))
    exe=output/(name+'_test.exe')
    subprocess.run(['rustc','--edition=2021','--test',str(generated),'-o',str(exe)],check=True)
    command=[str(exe)]
    if name=='before':command+=['explicit_zero_is_restored','--exact']
    r=subprocess.run(command,capture_output=True,text=True)
    (output/(name+'.log')).write_text(r.stdout+r.stderr)
    results[name]={'exit_code':r.returncode,'function_sha256':digest,'output':r.stdout}
assert results['before']['exit_code']!=0
assert results['patched']['exit_code']==0
results['status']='PASS'
results['scope']='Actual build function; registry, logs and SendSAS mocked. No real SAS calls.'
(output/'TEST_RESULT.json').write_text(json.dumps(results,indent=2))
print(json.dumps(results,indent=2))

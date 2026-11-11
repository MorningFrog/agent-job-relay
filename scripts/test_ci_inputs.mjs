import {test} from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {execFileSync,spawnSync} from 'node:child_process';
import {snapshot,restore} from '../.ci/ci_inputs.mjs';
function fixture(t) {
  const root=fs.mkdtempSync(path.join(process.cwd(),'.tmp/input-build-'));
  t.after(()=>fs.rmSync(root,{recursive:true,force:true})); return root;
}
test('Cargo reuses a moved checkout and rebuilds changed source/assets after content-checked restoration',t=>{
  const root=fixture(t), first=path.join(root,'first'), next=path.join(root,'next'), target=path.join(root,'target');
  fs.mkdirSync(first);execFileSync('git',['init','-q'],{cwd:first});
  fs.mkdirSync(path.join(first,'src'));fs.mkdirSync(path.join(first,'web/dist'),{recursive:true});
  // Exercise the real Windows stack-link instruction for the real binary name.
  fs.writeFileSync(path.join(first,'Cargo.toml'),'[package]\nname="agent-job-relay"\nversion="0.0.0"\nedition="2021"\n');
  fs.copyFileSync(new URL('../build.rs',import.meta.url),path.join(first,'build.rs'));
  fs.writeFileSync(path.join(first,'src/main.rs'),'include!(concat!(env!("OUT_DIR"),"/web_assets.rs")); fn main(){println!("{}",std::str::from_utf8(ASSETS[0].1).unwrap());}\n');
  fs.writeFileSync(path.join(first,'web/dist/index.html'),'initial');
  execFileSync('git',['add','.'],{cwd:first});
  const build=dir=>{
    const result=spawnSync('cargo',['build','--offline','--message-format=json'],{
      cwd:dir,env:{...process.env,CARGO_TARGET_DIR:target},encoding:'utf8'});
    assert.equal(result.status,0,result.stderr);
    return result.stdout.trim().split('\n').map(v=>JSON.parse(v)).filter(v=>v.reason==='compiler-artifact');
  };
  build(first); const previous=snapshot(first,['web/dist']);
  fs.cpSync(first,next,{recursive:true});fs.rmSync(first,{recursive:true,force:true});
  restore(next,snapshot(next,['web/dist']),previous);
  assert(build(next).every(v=>v.fresh));
  // A source-only edit must still compile after its old checkout disappeared.
  fs.appendFileSync(path.join(next,'src/main.rs'),'\n// changed native input\n');
  restore(next,snapshot(next,['web/dist']),previous);
  assert(build(next).some(v=>!v.fresh));
  fs.writeFileSync(path.join(next,'web/dist/index.html'),'changed asset');
  restore(next,snapshot(next,['web/dist']),previous);
  assert(build(next).some(v=>!v.fresh));
  const executable=path.join(target,'debug','agent-job-relay'+(process.platform==='win32'?'.exe':''));
  assert.equal(execFileSync(executable,{encoding:'utf8'}).trim(),'changed asset');
});

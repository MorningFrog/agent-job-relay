// SPDX-License-Identifier: MIT
// Content-checked input times for a dedicated, single-writer compiler cache.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {pathToFileURL} from 'node:url';

const need = (ok, message) => { if (!ok) throw new Error(message); };
function safeName(name) {
  return typeof name === 'string' && name && !name.includes('\\') && !name.includes(':') &&
    !name.startsWith('/') && name.split('/').every(p => p && p !== '.' && p !== '..' && p !== '.git');
}
function noLinks(file) {
  for (let current = path.resolve(file); ; current = path.dirname(current)) {
    const info = fs.lstatSync(current, {throwIfNoEntry:false});
    need(!info?.isSymbolicLink(), 'Symlinked compiler input/cache path refused');
    if (path.dirname(current) === current) break;
  }
}
function inside(root, name) {
  need(safeName(name), 'Compiler input paths must stay inside the checkout');
  const file = path.join(root, name);
  noLinks(file);
  return file;
}
export function snapshot(root, generated) {
  root = path.resolve(root); noLinks(root);
  need(Array.isArray(generated) && generated.length <= 16 && generated.every(safeName) &&
    new Set(generated).size === generated.length, 'Invalid generated-input directories');
  const tracked = execFileSync('git', ['ls-files','-z'], {cwd:root,encoding:'utf8',maxBuffer:32*1024*1024}).split('\0').filter(Boolean);
  const names = new Set(tracked), directories = Object.create(null);
  function walk(name) {
    const file = inside(root, name), info = fs.lstatSync(file, {throwIfNoEntry:false});
    if (!info) return;
    need(info.isDirectory(), 'Generated input root must be a directory');
    directories[name] = info.mtimeMs;
    for (const entry of fs.readdirSync(file, {withFileTypes:true})) {
      const child = name+'/'+entry.name;
      need(!entry.isSymbolicLink(), 'Generated input symlink refused');
      if (entry.isDirectory()) walk(child);
      else { need(entry.isFile(), 'Unsupported generated input'); names.add(child); }
    }
  }
  generated.forEach(walk);
  const files = Object.create(null);
  for (const name of [...names].sort()) {
    const file = inside(root, name), before = fs.lstatSync(file, {throwIfNoEntry:false});
    if (!before) continue;
    need(before.isFile(), 'Unsupported tracked compiler input');
    const bytes = fs.readFileSync(file), after = fs.statSync(file);
    need(before.size === after.size && before.mtimeMs === after.mtimeMs && before.mode === after.mode,
      'Compiler input changed while hashing');
    files[name] = {sha256:crypto.createHash('sha256').update(bytes).digest('hex'), mode:before.mode & 0o777,
      size:before.size, mtime:before.mtimeMs};
  }
  return {format:1, platform:process.platform, arch:process.arch, recorded:Date.now(), generated, files, directories};
}
export function validate(value) {
  need(value?.format === 1 && typeof value.platform === 'string' && typeof value.arch === 'string' &&
    Number.isFinite(value.recorded) && value.recorded > 0 && value.recorded <= 8.64e15 && Array.isArray(value.generated) &&
    value.generated.every(safeName) && value.files && value.directories &&
    typeof value.files === 'object' && !Array.isArray(value.files) &&
    typeof value.directories === 'object' && !Array.isArray(value.directories), 'Invalid compiler-input manifest');
  need(Object.keys(value.files).length <= 100000, 'Compiler-input inventory too large');
  for (const [name, file] of Object.entries(value.files)) need(safeName(name) && file &&
    /^[0-9a-f]{64}$/.test(file.sha256) && Number.isInteger(file.mode) && file.mode >= 0 && file.mode <= 0o777 &&
    Number.isSafeInteger(file.size) && file.size >= 0 && Number.isFinite(file.mtime) && file.mtime >= 0 && file.mtime <= 8.64e15,
    'Invalid compiler-input file record');
  for (const [name, time] of Object.entries(value.directories)) need(safeName(name) &&
    value.generated.some(root => name === root || name.startsWith(root+'/')) && Number.isFinite(time) && time >= 0 && time <= 8.64e15,
    'Invalid generated-directory record');
  return value;
}
const sameFile = (a,b) => a && b && a.sha256 === b.sha256 && a.mode === b.mode && a.size === b.size;
export function restore(root, current, previous) {
  validate(previous); validate(current);
  if (current.platform !== previous.platform || current.arch !== previous.arch ||
      JSON.stringify(current.generated) !== JSON.stringify(previous.generated))
    return {restored:0, changed:0, reason:'environment_or_generated_roots_changed'};
  const fresh = Math.max(Date.now(), previous.recorded+2);
  const changes = [];
  let restored = 0;
  for (const [name, file] of Object.entries(current.files)) {
    const same = sameFile(file, previous.files[name]);
    changes.push([name, same ? previous.files[name].mtime : fresh]);
    if (same) restored++;
  }
  for (const directory of Object.keys(current.directories)) {
    const prefix = directory+'/', names = Object.keys(current.files).filter(n => n.startsWith(prefix));
    const oldNames = Object.keys(previous.files).filter(n => n.startsWith(prefix));
    const same = previous.directories[directory] !== undefined && names.length === oldNames.length &&
      names.every(n => sameFile(current.files[n], previous.files[n])) &&
      Object.keys(current.directories).filter(n => n.startsWith(prefix)).sort().join('\0') ===
      Object.keys(previous.directories).filter(n => n.startsWith(prefix)).sort().join('\0');
    changes.push([directory, same ? previous.directories[directory] : fresh]);
  }
  // Validate every path before changing even one timestamp.
  const targets = changes.map(([name, time]) => [inside(root,name), time]);
  for (const [file, time] of targets) fs.utimesSync(file, fs.statSync(file).atimeMs/1000, time/1000);
  return {restored, changed:Object.keys(current.files).length-restored, reason:'content_and_modes_checked'};
}

function main() {
  const [operation] = process.argv.slice(2);
  need(['record','restore'].includes(operation), 'Usage: node .ci/ci_inputs.mjs {restore|record}');
  const root = process.cwd(), file = process.env.WP_INPUT_MTIMES_FILE;
  need(file && path.isAbsolute(file), 'Set an absolute WP_INPUT_MTIMES_FILE in the dedicated cache');
  const config = JSON.parse(fs.readFileSync('.ci/input-mtimes.json','utf8'));
  need(config && Object.keys(config).length === 1 && Array.isArray(config.generated), 'Invalid compiler-input configuration');
  noLinks(file);
  const current = snapshot(root, config.generated);
  const exists = fs.existsSync(file);
  let previous;
  if (exists) previous = validate(JSON.parse(fs.readFileSync(file,'utf8')));
  let result;
  if (operation === 'restore') result = previous ? restore(root,current,previous) : {restored:0,reason:'no_previous_manifest'};
  else {
    need(!Object.keys(current.files).some(name => path.resolve(root,name) === path.resolve(file)), 'Cache manifest cannot overwrite source');
    fs.mkdirSync(path.dirname(file), {recursive:true}); noLinks(file);
    const temporary = file+'.'+crypto.randomUUID()+'.tmp';
    const fd = fs.openSync(temporary,'wx',0o600);
    try { fs.writeFileSync(fd,JSON.stringify(current)+'\n'); fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
    fs.renameSync(temporary,file);
    result = {recorded:Object.keys(current.files).length,reason:'compiler_inputs_only'};
  }
  console.log(JSON.stringify({input_mtimes:operation,...result}));
}
if (process.argv[1] && pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url) {
  try { main(); }
  catch { console.log(JSON.stringify({input_mtimes:process.argv[2],restored:0,reason:'unsafe_or_unavailable_inputs_preserved'})); }
}

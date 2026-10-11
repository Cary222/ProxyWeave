// Non-sensitive, explicit three-file package. Never traverses source auths/data/config/logs.
// Fingerprints are from the audited artifact, not an arbitrary new binary.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const [sourceDirectory, packageDirectory] = process.argv.slice(2);
if (!sourceDirectory || !packageDirectory) {
  throw new Error('Usage: node prepare-wb2api-package.cjs <audited-wb2api-directory> <new-package-directory>');
}
const manifest = path.resolve(__dirname, '../../manifests/wb2api.manifest.yaml');
const inputs = [
  [path.join(sourceDirectory, 'wb2api.exe'), 'bin/wb2api.exe', '38c4d8e0421c6c8c24ec738459899c13d07e9a56730db3394a4021e460c1d485'],
  [path.join(sourceDirectory, 'LICENSE'), 'LICENSE', '49b9e27ddfc7cceab55e19eb7d405bda30a8229de3cc094e6b4d0397ab76a34b'],
  [manifest, 'manifest.yaml', '2d237912d2a396cd39143251d3985250308444bd50551a7073f4955fd9cf5a76'],
];
const bytes = inputs.map(([file, relative, expected]) => {
  const data = fs.readFileSync(file);
  const sha256 = crypto.createHash('sha256').update(data).digest('hex');
  if (sha256 !== expected) throw new Error(`Audited artifact changed: ${relative}`);
  return { relative, data, sha256 };
});
const target = path.resolve(packageDirectory);
if (fs.existsSync(target)) throw new Error('Destination must be new; no package overwrite');
fs.mkdirSync(target, { recursive: true });
for (const { relative, data, sha256 } of bytes) {
  const destination = path.join(target, relative);
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  fs.writeFileSync(destination, data, { flag: 'wx' });
  console.log(`${relative} SHA256=${sha256}`);
}

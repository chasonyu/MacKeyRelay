#!/usr/bin/env node
'use strict';

// Postinstall: download the prebuilt mackeyrelay binary for the current
// platform/arch from GitHub Releases into ./vendor, so the bin wrapper
// (bin/mackeyrelay.js) can spawn it. No binary ships in the npm tarball.

const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');

const REPO = 'chasonyu/MacKeyRelay';

const TARGETS = {
  darwin: { arm64: 'darwin-arm64' },
};

function die(msg) {
  console.error(`[mackeyrelay postinstall] ${msg}`);
  process.exit(0); // don't fail the whole npm install on unsupported env
}

const suffix = (TARGETS[process.platform] || {})[process.arch];
if (!suffix) {
  die(`no prebuilt binary for ${process.platform}/${process.arch}. Build from source: https://github.com/${REPO}`);
}

const { version } = require('../package.json');
const tag = `v${version}`;
const asset = `mackeyrelay-${tag}-${suffix}.tar.gz`;
const url = `https://github.com/${REPO}/releases/download/${tag}/${asset}`;

const vendorDir = path.join(__dirname, '..', 'vendor');
const tarball = path.join(vendorDir, asset);
const binPath = path.join(vendorDir, 'mackeyrelay');

fs.mkdirSync(vendorDir, { recursive: true });

// Use curl instead of node's https: curl transparently honors HTTP(S)_PROXY /
// ALL_PROXY env vars, which is required behind corporate proxies.
console.log(`[mackeyrelay postinstall] downloading ${asset}`);
try {
  execFileSync('curl', ['-fsSL', '--max-time', '180', '--retry', '2', '-o', tarball, url], { stdio: 'inherit' });
} catch (e) {
  die(`download failed: ${e.message}`);
}

try {
  execFileSync('tar', ['-xzf', tarball, '-C', vendorDir], { stdio: 'inherit' });
} catch (e) {
  die(`extraction failed: ${e.message}`);
}
fs.rmSync(tarball, { force: true });

if (!fs.existsSync(binPath)) {
  die(`binary not found after extraction (expected ${binPath})`);
}
fs.chmodSync(binPath, 0o755);
console.log(`[mackeyrelay postinstall] ready at ${binPath}`);

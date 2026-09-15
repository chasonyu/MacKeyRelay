#!/usr/bin/env node
'use strict';

const { spawnSync } = require('child_process');
const { join } = require('path');

const bin = join(__dirname, '..', 'vendor', 'mackeyrelay');
const result = spawnSync(bin, process.argv.slice(2), { stdio: 'inherit' });

const sig = result.signal;
if (sig) {
  process.kill(process.pid, sig);
}
process.exit(result.status == null ? 1 : result.status);

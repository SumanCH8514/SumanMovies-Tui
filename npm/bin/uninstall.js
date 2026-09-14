#!/usr/bin/env node

const { uninstall } = require('../lib/installer');

const args = process.argv.slice(2);
const purge = args.includes('--purge') || args.includes('-p');

uninstall({ purge });

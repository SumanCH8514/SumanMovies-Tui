const { uninstall } = require('../lib/installer');

try {
  const args = process.argv.slice(2);
  const purge = args.includes('--purge') || args.includes('-p');
  uninstall({ purge });
} catch (err) {}

const { update } = require('../lib/installer');

(async () => {
  try {
    await update();
  } catch (err) {
    console.error('Update failed:', err.message);
    process.exit(1);
  }
})();

const { setupAll } = require('../lib/installer');

(async () => {
  try {
    console.log('📦 Installing SumanMovies prerequisites...');
    await setupAll();
    console.log('✅ SumanMovies ready to use! Run `sumanmovies` in your terminal.');
  } catch (err) {
    console.log('ℹ️ First-time setup will complete automatically on first run (`sumanmovies`).');
  }
})();

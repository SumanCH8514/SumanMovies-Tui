#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');
const { setupAll, findExecutable, getStorageDir, update, uninstall } = require('../lib/installer');

async function main() {
  try {
    const args = process.argv.slice(2);
    if (args.includes('--update') || args.includes('update') || args.includes('-u')) {
      await update();
      process.exit(0);
    }
    if (args.includes('--uninstall') || args.includes('uninstall')) {
      const purge = args.includes('--purge') || args.includes('-p');
      uninstall({ purge });
      process.exit(0);
    }

    const storageDir = getStorageDir();
    const setupMarker = path.join(storageDir, '.initialized');
    let binPath = findExecutable('sumanmovies');

    if (!binPath || !fs.existsSync(setupMarker)) {
      console.log('\n  🎬 SumanMovies - Initializing required dependencies...');
      binPath = await setupAll();
      try {
        fs.writeFileSync(setupMarker, new Date().toISOString(), 'utf8');
      } catch (_) {}
      console.log('  ✨ Setup complete!\n');
    }

    if (!binPath) {
      binPath = findExecutable('sumanmovies');
    }

    if (!binPath) {
      throw new Error('SumanMovies binary could not be found or executed. Try running: sumanmovies --update');
    }

    const currentPath = process.env.PATH || '';
    if (!currentPath.includes(storageDir)) {
      process.env.PATH = `${storageDir}${path.delimiter}${currentPath}`;
    }

    const inWezterm = process.env.TERM_PROGRAM === 'WezTerm' || !!process.env.WEZTERM_PANE;
    const wantsWezterm = args.includes('--wezterm') || args.includes('-w') || process.env.SUMANMOVIES_WEZTERM === '1';
    const noWezterm = args.includes('--no-wezterm') || args.includes('-nw');

    if (wantsWezterm && !inWezterm && !noWezterm) {
      const weztermBin = findExecutable('wezterm');
      if (weztermBin) {
        const passArgs = args.filter(a => a !== '--wezterm' && a !== '-w');
        const child = spawn(weztermBin, ['start', '--', binPath, ...passArgs], {
          stdio: 'inherit',
          windowsHide: false
        });
        child.on('exit', (code) => process.exit(code || 0));
        return;
      }
    }

    const child = spawn(binPath, args, {
      stdio: 'inherit',
      env: process.env,
      windowsHide: false
    });

    child.on('error', (err) => {
      console.error('Failed to start SumanMovies:', err);
      process.exit(1);
    });

    child.on('exit', (code, signal) => {
      if (signal) {
        process.kill(process.pid, signal);
      } else {
        process.exit(code || 0);
      }
    });

    process.on('SIGINT', () => {
      child.kill('SIGINT');
    });
    process.on('SIGTERM', () => {
      child.kill('SIGTERM');
    });
  } catch (error) {
    console.error('Error launching SumanMovies:', error.message);
    process.exit(1);
  }
}

main();

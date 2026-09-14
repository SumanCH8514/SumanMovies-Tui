#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
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
    let binPath = findExecutable('sumanmovies');
    let mpvPath = findExecutable('mpv');
    let ytdlpPath = findExecutable('yt-dlp');
    let ffmpegPath = findExecutable('ffmpeg');

    if (!binPath || !mpvPath || !ytdlpPath || !ffmpegPath) {
      console.log('\n  🎬 SumanMovies - Initializing required dependencies...');
      binPath = await setupAll();
      console.log('  ✨ Setup complete!\n');
    }

    const storageDir = getStorageDir();
    const currentPath = process.env.PATH || '';
    if (!currentPath.includes(storageDir)) {
      process.env.PATH = `${storageDir}${path.delimiter}${currentPath}`;
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

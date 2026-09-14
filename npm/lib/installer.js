const fs = require('fs');
const path = require('path');
const os = require('os');
const https = require('https');
const http = require('http');
const { execSync } = require('child_process');

const REPO = 'SumanCH8514/SumanMovies-Tui';

function getStorageDir() {
  const home = os.homedir();
  const dir = path.join(home, '.sumanmovies', 'bin');
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
  return dir;
}

function findExecutable(name) {
  const isWin = process.platform === 'win32';
  const candNames = isWin ? [`${name}.exe`, `${name}.com`, name] : [name];
  if (name === 'sumanmovies') {
    candNames.push(isWin ? 'sumanmovies-tui.exe' : 'sumanmovies-tui');
  }

  for (const binName of candNames) {
    const localPkgBin = path.join(__dirname, '..', 'dist', binName);
    if (fs.existsSync(localPkgBin)) return localPkgBin;

    const userBin = path.join(getStorageDir(), binName);
    if (fs.existsSync(userBin)) return userBin;

    if (isWin && process.env.LOCALAPPDATA) {
      const appDataBin = path.join(process.env.LOCALAPPDATA, 'Programs', 'SumanMovies', 'bin', binName);
      if (fs.existsSync(appDataBin)) return appDataBin;
    }

    const localBin = path.join(os.homedir(), '.local', 'bin', binName);
    if (fs.existsSync(localBin)) return localBin;

    const pathDirs = (process.env.PATH || '').split(path.delimiter);
    for (const dir of pathDirs) {
      const fullPath = path.join(dir, binName);
      if (fs.existsSync(fullPath)) return fullPath;
    }
  }

  return null;
}

function downloadFile(url, destPath) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(destPath);
    const client = url.startsWith('https') ? https : http;

    const request = client.get(url, {
      headers: {
        'User-Agent': 'SumanMovies-NPM-Installer'
      }
    }, (response) => {
      if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
        file.close();
        if (fs.existsSync(destPath)) {
          try { fs.unlinkSync(destPath); } catch (_) {}
        }
        return downloadFile(response.headers.location, destPath).then(resolve).catch(reject);
      }

      if (response.statusCode !== 200) {
        file.close();
        if (fs.existsSync(destPath)) {
          try { fs.unlinkSync(destPath); } catch (_) {}
        }
        return reject(new Error(`Failed to download: HTTP ${response.statusCode}`));
      }

      response.pipe(file);
      file.on('finish', () => {
        file.close(resolve);
      });
      file.on('error', (err) => {
        file.close();
        if (fs.existsSync(destPath)) {
          try { fs.unlinkSync(destPath); } catch (_) {}
        }
        reject(err);
      });
    });

    request.on('error', (err) => {
      file.close();
      if (fs.existsSync(destPath)) {
        try { fs.unlinkSync(destPath); } catch (_) {}
      }
      reject(err);
    });
  });
}

async function ensureSumanMoviesBinary(force = false) {
  const isWin = process.platform === 'win32';
  const binName = isWin ? 'sumanmovies.exe' : 'sumanmovies';
  const altBinName = isWin ? 'sumanmovies-tui.exe' : 'sumanmovies-tui';
  const storageDir = getStorageDir();
  const targetPath = path.join(storageDir, binName);
  const altPath = path.join(storageDir, altBinName);

  if (!force) {
    if (fs.existsSync(targetPath)) return targetPath;
    if (fs.existsSync(altPath)) return altPath;
  }

  if (force) {
    for (const p of [targetPath, altPath]) {
      if (fs.existsSync(p)) {
        try { fs.unlinkSync(p); } catch (e) {}
      }
    }
  }

  const localDist = path.join(__dirname, '..', 'dist', binName);
  if (fs.existsSync(localDist)) {
    fs.copyFileSync(localDist, targetPath);
    if (!isWin) fs.chmodSync(targetPath, 0o755);
    return targetPath;
  }

  if (isWin && process.env.LOCALAPPDATA) {
    const localBuilt = path.join(process.env.LOCALAPPDATA, 'Programs', 'SumanMovies', 'bin', binName);
    if (fs.existsSync(localBuilt)) {
      fs.copyFileSync(localBuilt, targetPath);
      return targetPath;
    }
  }

  const workspaceRelease = path.resolve(__dirname, '..', '..', 'target', 'release', binName);
  if (fs.existsSync(workspaceRelease)) {
    fs.copyFileSync(workspaceRelease, targetPath);
    if (!isWin) fs.chmodSync(targetPath, 0o755);
    return targetPath;
  }

  console.log('  > Downloading SumanMovies binary...');
  try {
    const isMac = process.platform === 'darwin';
    const isAndroid = process.platform === 'android' || fs.existsSync('/data/data/com.termux');
    const arch = process.arch === 'arm64' ? 'arm64' : 'x64';

    let archiveName;
    if (isWin) {
      archiveName = `SumanMovies_Windows_${arch}.zip`;
    } else if (isMac) {
      archiveName = 'SumanMovies_macOS_Universal.tar.gz';
    } else if (isAndroid) {
      archiveName = `SumanMovies_Android_${arch}.tar.gz`;
    } else {
      archiveName = `SumanMovies_Linux_${arch}.tar.gz`;
    }

    const isTarGz = archiveName.endsWith('.tar.gz');
    const tempFile = path.join(storageDir, isTarGz ? 'temp.tar.gz' : 'temp.zip');

    // Clean up any stale temp files before download
    for (const stale of ['temp.zip', 'temp.tar.gz', 'temp']) {
      const sp = path.join(storageDir, stale);
      if (fs.existsSync(sp)) {
        try { fs.unlinkSync(sp); } catch (_) {}
      }
    }

    const downloadUrl = `https://github.com/${REPO}/releases/latest/download/${archiveName}`;
    await downloadFile(downloadUrl, tempFile);

    if (isWin) {
      execSync(`powershell -NoProfile -Command "Expand-Archive -Path '${tempFile}' -DestinationPath '${storageDir}' -Force"`, { stdio: 'ignore' });
      for (const cand of ['sumanmovies-tui.exe', 'sumanmovies.exe']) {
        const extracted = path.join(storageDir, cand);
        if (fs.existsSync(extracted)) {
          if (extracted !== targetPath && !fs.existsSync(targetPath)) {
            try { fs.copyFileSync(extracted, targetPath); } catch (_) {}
          }
          break;
        }
      }
    } else {
      if (isTarGz) {
        execSync(`tar -xzf "${tempFile}" -C "${storageDir}"`, { stdio: 'ignore' });
      } else {
        try {
          execSync(`tar -xf "${tempFile}" -C "${storageDir}"`, { stdio: 'ignore' });
        } catch (_) {
          execSync(`unzip -o "${tempFile}" -d "${storageDir}"`, { stdio: 'ignore' });
        }
      }

      for (const cand of ['sumanmovies', 'sumanmovies-tui']) {
        const extracted = path.join(storageDir, cand);
        if (fs.existsSync(extracted)) {
          try { fs.chmodSync(extracted, 0o755); } catch (_) {}
          if (extracted !== targetPath && !fs.existsSync(targetPath)) {
            try {
              fs.copyFileSync(extracted, targetPath);
              fs.chmodSync(targetPath, 0o755);
            } catch (_) {}
          }
        }
      }
    }

    if (fs.existsSync(tempFile)) {
      try { fs.unlinkSync(tempFile); } catch (_) {}
    }

    if (fs.existsSync(targetPath)) {
      console.log('  + SumanMovies binary configured.');
      return targetPath;
    }
    if (fs.existsSync(altPath)) {
      console.log('  + SumanMovies binary configured.');
      return altPath;
    }

    throw new Error(`Extraction finished but binary not found at ${targetPath}`);
  } catch (e) {
    throw new Error(`Could not download or install ${binName}: ${e.message}`);
  }
}

async function ensureMpv() {
  const existingMpv = findExecutable('mpv');
  if (existingMpv) {
    return existingMpv;
  }

  const storageDir = getStorageDir();
  const isWin = process.platform === 'win32';

  if (isWin) {
    console.log('  > Setting up MPV media player for Windows...');
    
    const candidatePaths = [
      path.join(os.homedir(), '.local', 'bin', 'mpv.exe'),
      path.join(os.homedir(), '.local', 'bin', 'mpv.com'),
      'C:\\Program Files\\mpv\\mpv.exe',
      'C:\\mpv\\mpv.exe',
      path.join(process.env.LOCALAPPDATA || '', 'Programs', 'mpv', 'mpv.exe'),
    ];

    for (const cand of candidatePaths) {
      if (fs.existsSync(cand)) {
        const destMpv = path.join(storageDir, path.basename(cand));
        try {
          fs.copyFileSync(cand, destMpv);
          console.log('  + MPV player configured from local installation.');
          return destMpv;
        } catch (e) {}
      }
    }

    try {
      console.log('  > Installing MPV via winget...');
      execSync('winget install --id shinchiro.mpv --silent --accept-source-agreements --accept-package-agreements', { stdio: 'ignore' });
      const found = findExecutable('mpv');
      if (found) {
        console.log('  + MPV player installed via winget.');
        return found;
      }
    } catch (e) {}
  } else if (process.platform === 'darwin') {
    try {
      console.log('  > Installing MPV via Homebrew...');
      execSync('brew install mpv', { stdio: 'inherit' });
    } catch (e) {
      console.log('  ! Note: Please install mpv via Homebrew if not installed: brew install mpv');
    }
  } else {
    try {
      if (fs.existsSync('/data/data/com.termux/files/usr/bin/pkg')) {
        execSync('pkg install -y mpv', { stdio: 'inherit' });
      } else {
        execSync('sudo apt-get install -y mpv', { stdio: 'inherit' });
      }
    } catch (e) {
      console.log('  ! Note: Please install mpv via your package manager (e.g. sudo apt install mpv)');
    }
  }

  return findExecutable('mpv');
}

async function ensureYtDlp() {
  const existing = findExecutable('yt-dlp');
  if (existing) {
    return existing;
  }

  const storageDir = getStorageDir();
  const isWin = process.platform === 'win32';
  const binName = isWin ? 'yt-dlp.exe' : 'yt-dlp';
  const destPath = path.join(storageDir, binName);

  if (fs.existsSync(destPath)) {
    return destPath;
  }

  console.log('  > Setting up yt-dlp (stream & media downloader)...');

  if (isWin) {
    const candidatePaths = [
      path.join(os.homedir(), '.local', 'bin', 'yt-dlp.exe'),
      path.join(process.env.LOCALAPPDATA || '', 'Programs', 'Python', 'Python313', 'Scripts', 'yt-dlp.exe'),
      path.join(process.env.LOCALAPPDATA || '', 'Programs', 'Python', 'Python312', 'Scripts', 'yt-dlp.exe'),
      path.join(process.env.LOCALAPPDATA || '', 'Programs', 'Python', 'Python311', 'Scripts', 'yt-dlp.exe'),
      path.join(process.env.APPDATA || '', 'Python', 'Scripts', 'yt-dlp.exe'),
    ];
    for (const cand of candidatePaths) {
      if (fs.existsSync(cand)) {
        try {
          fs.copyFileSync(cand, destPath);
          console.log('  + yt-dlp configured successfully from local copy.');
          return destPath;
        } catch (e) {}
      }
    }
  }

  try {
    const downloadUrl = isWin
      ? 'https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe'
      : (process.platform === 'darwin'
          ? 'https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos'
          : 'https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp');

    console.log('  > Downloading latest standalone yt-dlp...');
    await downloadFile(downloadUrl, destPath);
    if (!isWin) {
      fs.chmodSync(destPath, 0o755);
    }
    console.log('  + yt-dlp downloaded and configured.');
    return destPath;
  } catch (e) {
    if (isWin) {
      try {
        execSync('winget install --id yt-dlp.yt-dlp --silent --accept-source-agreements --accept-package-agreements', { stdio: 'ignore' });
      } catch (err) {}
    } else if (process.platform === 'darwin') {
      try {
        execSync('brew install yt-dlp', { stdio: 'inherit' });
      } catch (err) {}
    } else {
      try {
        if (fs.existsSync('/data/data/com.termux/files/usr/bin/pkg')) {
          execSync('pkg install -y yt-dlp', { stdio: 'inherit' });
        }
      } catch (err) {}
    }
  }

  return findExecutable('yt-dlp');
}

async function ensureFfmpeg() {
  const existing = findExecutable('ffmpeg');
  if (existing) {
    return existing;
  }

  const storageDir = getStorageDir();
  const isWin = process.platform === 'win32';
  const binName = isWin ? 'ffmpeg.exe' : 'ffmpeg';
  const destPath = path.join(storageDir, binName);

  if (fs.existsSync(destPath)) {
    return destPath;
  }

  console.log('  > Setting up ffmpeg (media processor & stream merger)...');

  if (isWin) {
    const candidateDirs = [
      'C:\\ffmpeg\\bin',
      'C:\\Program Files\\ffmpeg\\bin',
      path.join(os.homedir(), '.local', 'bin'),
      path.join(process.env.LOCALAPPDATA || '', 'Programs', 'ffmpeg', 'bin'),
    ];
    for (const dir of candidateDirs) {
      const cand = path.join(dir, 'ffmpeg.exe');
      if (fs.existsSync(cand)) {
        try {
          fs.copyFileSync(cand, destPath);
          const ffprobeSrc = path.join(dir, 'ffprobe.exe');
          if (fs.existsSync(ffprobeSrc)) {
            fs.copyFileSync(ffprobeSrc, path.join(storageDir, 'ffprobe.exe'));
          }
          console.log('  + ffmpeg configured successfully from local copy.');
          return destPath;
        } catch (e) {}
      }
    }

    try {
      console.log('  > Installing ffmpeg via winget...');
      execSync('winget install --id Gyan.FFmpeg --silent --accept-source-agreements --accept-package-agreements', { stdio: 'ignore' });
      const found = findExecutable('ffmpeg');
      if (found) {
        console.log('  + ffmpeg installed via winget.');
        return found;
      }
    } catch (e) {}

    try {
      console.log('  > Downloading portable FFmpeg essentials...');
      const zipUrl = 'https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip';
      const tempZip = path.join(storageDir, 'ffmpeg_temp.zip');
      await downloadFile(zipUrl, tempZip);
      const tempExtract = path.join(storageDir, 'ffmpeg_extract');
      if (!fs.existsSync(tempExtract)) fs.mkdirSync(tempExtract, { recursive: true });
      execSync(`powershell -NoProfile -Command "Expand-Archive -Path '${tempZip}' -DestinationPath '${tempExtract}' -Force"`, { stdio: 'ignore' });

      function findFile(dir, target) {
        const files = fs.readdirSync(dir);
        for (const file of files) {
          const full = path.join(dir, file);
          if (fs.statSync(full).isDirectory()) {
            const res = findFile(full, target);
            if (res) return res;
          } else if (file.toLowerCase() === target.toLowerCase()) {
            return full;
          }
        }
        return null;
      }

      const extractedFfmpeg = findFile(tempExtract, 'ffmpeg.exe');
      if (extractedFfmpeg) {
        fs.copyFileSync(extractedFfmpeg, destPath);
      }
      const extractedFfprobe = findFile(tempExtract, 'ffprobe.exe');
      if (extractedFfprobe) {
        fs.copyFileSync(extractedFfprobe, path.join(storageDir, 'ffprobe.exe'));
      }
      fs.rmSync(tempExtract, { recursive: true, force: true });
      if (fs.existsSync(tempZip)) fs.unlinkSync(tempZip);
      console.log('  + ffmpeg downloaded and configured.');
      return destPath;
    } catch (e) {
      console.log('  ! Note: ffmpeg can also be installed manually (e.g. winget install Gyan.FFmpeg)');
    }
  } else if (process.platform === 'darwin') {
    try {
      console.log('  > Installing ffmpeg via Homebrew...');
      execSync('brew install ffmpeg', { stdio: 'inherit' });
    } catch (e) {
      console.log('  ! Note: Please install ffmpeg via Homebrew: brew install ffmpeg');
    }
  } else {
    try {
      if (fs.existsSync('/data/data/com.termux/files/usr/bin/pkg')) {
        execSync('pkg install -y ffmpeg', { stdio: 'inherit' });
      } else {
        execSync('sudo apt-get install -y ffmpeg', { stdio: 'inherit' });
      }
    } catch (e) {
      console.log('  ! Note: Please install ffmpeg via your package manager (e.g. sudo apt install ffmpeg)');
    }
  }

  return findExecutable('ffmpeg');
}

async function ensureWezTerm() {
  const existing = findExecutable('wezterm');
  if (existing) {
    return existing;
  }

  const storageDir = getStorageDir();
  const isWin = process.platform === 'win32';
  const isMac = process.platform === 'darwin';

  if (isWin) {
    const candidatePaths = [
      'C:\\Program Files\\WezTerm\\wezterm.exe',
      'C:\\Program Files (x86)\\WezTerm\\wezterm.exe',
      path.join(process.env.LOCALAPPDATA || '', 'Programs', 'WezTerm', 'wezterm.exe'),
      path.join(process.env.LOCALAPPDATA || '', 'Microsoft', 'WinGet', 'Links', 'wezterm.exe'),
      path.join(os.homedir(), '.local', 'bin', 'wezterm.exe'),
    ];
    for (const cand of candidatePaths) {
      if (fs.existsSync(cand)) {
        return cand;
      }
    }

    try {
      console.log('  > Setting up WezTerm (GPU-accelerated terminal for HD artwork)...');
      execSync('winget install --id wez.wezterm --silent --accept-source-agreements --accept-package-agreements', { stdio: 'ignore' });
      for (const cand of candidatePaths) {
        if (fs.existsSync(cand)) {
          console.log('  + WezTerm installed successfully via winget.');
          return cand;
        }
      }
      const found = findExecutable('wezterm');
      if (found) {
        console.log('  + WezTerm installed successfully via winget.');
        return found;
      }
    } catch (e) {
      console.log('  ! Note: WezTerm can also be installed manually (winget install wez.wezterm)');
    }
  } else if (isMac) {
    try {
      console.log('  > Installing WezTerm via Homebrew...');
      execSync('brew install --cask wezterm', { stdio: 'inherit' });
      return findExecutable('wezterm');
    } catch (e) {
      console.log('  ! Note: Install WezTerm via: brew install --cask wezterm');
    }
  } else {
    // Linux
    console.log('  > Setting up WezTerm (GPU-accelerated terminal for HD artwork)...');
    try {
      // 1. Try apt repository if on Debian/Ubuntu
      if (fs.existsSync('/usr/bin/apt-get') || fs.existsSync('/bin/apt-get')) {
        try {
          execSync(
            'curl -fsSL https://apt.fury.io/wez/gpg.key | sudo gpg --yes --dearmor -o /usr/share/keyrings/wezterm-fury.gpg 2>/dev/null && ' +
            'echo "deb [signed-by=/usr/share/keyrings/wezterm-fury.gpg] https://apt.fury.io/wez/ * *" | sudo tee /etc/apt/sources.list.d/wezterm.list >/dev/null && ' +
            'sudo apt-get update -qq && sudo apt-get install -y wezterm',
            { stdio: 'inherit' }
          );
          const found = findExecutable('wezterm');
          if (found) {
            console.log('  + WezTerm installed successfully via apt.');
            return found;
          }
        } catch (_) {}
      }

      // 2. Try flatpak if available
      if (fs.existsSync('/usr/bin/flatpak')) {
        try {
          execSync('flatpak install -y flathub org.wezfurlong.wezterm', { stdio: 'ignore' });
        } catch (_) {}
      }

      // 3. Fallback: Download official portable AppImage directly to storageDir
      const appImagePath = path.join(storageDir, 'wezterm');
      if (!fs.existsSync(appImagePath)) {
        try {
          console.log('  > Downloading portable WezTerm AppImage for HD graphics...');
          const appImageUrl = 'https://github.com/wez/wezterm/releases/download/20240203-110809-50462022/WezTerm-20240203-110809-50462022-Ubuntu20.04.AppImage';
          await downloadFile(appImageUrl, appImagePath);
          fs.chmodSync(appImagePath, 0o755);
          console.log('  + Portable WezTerm downloaded and configured.');
          return appImagePath;
        } catch (err) {}
      } else {
        return appImagePath;
      }
    } catch (e) {
      console.log('  ! Note: WezTerm can be installed manually (e.g. sudo apt install wezterm)');
    }
  }

  return findExecutable('wezterm');
}

async function setupAll(force = false) {
  console.log('\n  📦 Initializing SumanMovies and all required dependencies...');
  
  const binPath = await ensureSumanMoviesBinary(force);
  const mpvPath = await ensureMpv();
  const ytdlpPath = await ensureYtDlp();
  const ffmpegPath = await ensureFfmpeg();
  const weztermPath = await ensureWezTerm();

  console.log('\n  ✨ All Required Dependencies Checked:');
  console.log(`    • SumanMovies Engine : ${binPath ? '✓ Installed' : '✗ Missing'}`);
  console.log(`    • MPV Media Player   : ${mpvPath ? '✓ Ready' : '⚠ Missing (playback requires mpv or vlc)'}`);
  console.log(`    • yt-dlp Downloader  : ${ytdlpPath ? '✓ Ready' : '⚠ Missing (required for DASH stream downloads)'}`);
  console.log(`    • FFmpeg Processor   : ${ffmpegPath ? '✓ Ready' : '⚠ Missing (required for audio/video merge)'}`);
  console.log(`    • WezTerm HD Terminal: ${weztermPath ? '✓ Ready (Full-HD Poster Artwork Enabled)' : '⚠ Optional (Install for High-Definition Posters)'}\n`);

  return binPath;
}

async function update() {
  console.log('\n  🔄 Checking for SumanMovies updates...');
  
  let currentVersion = '1.0.4';
  try {
    const pkg = require('../package.json');
    currentVersion = pkg.version;
  } catch (e) {}
  console.log(`  Current installed package: v${currentVersion}`);

  try {
    console.log('  > Checking latest version on npm registry...');
    const latestVersion = execSync('npm view sumanmovies version', { encoding: 'utf8', stdio: ['pipe', 'pipe', 'ignore'] }).trim();
    if (latestVersion) {
      console.log(`  Latest published version:  v${latestVersion}`);
      if (latestVersion !== currentVersion) {
        console.log('\n  📦 Updating npm package globally...');
        try {
          execSync('npm install -g sumanmovies@latest', { stdio: 'inherit' });
          console.log('  ✓ Global npm package updated.');
        } catch (err) {
          console.log('  ! If you installed with different permissions, run: npm install -g sumanmovies@latest');
        }
      } else {
        console.log('  ✓ NPM package is up to date.');
      }
    }
  } catch (e) {}

  const ytdlp = findExecutable('yt-dlp');
  if (ytdlp) {
    try {
      console.log('  > Checking yt-dlp updates...');
      execSync(`"${ytdlp}" -U`, { stdio: 'ignore' });
      console.log('  ✓ yt-dlp verified.');
    } catch (e) {}
  }

  console.log('  > Refreshing SumanMovies engine & dependencies...');
  const binPath = await setupAll(true);
  console.log('  ✨ SumanMovies is fully up to date! Run `sumanmovies` to start.\n');
  return binPath;
}

function uninstall(options = {}) {
  const purgeData = !!(options && (options.purge || options.all));
  console.log('\n  🗑️  Uninstalling SumanMovies...');

  const home = os.homedir();
  const storageDir = path.join(home, '.sumanmovies');
  if (fs.existsSync(storageDir)) {
    try {
      fs.rmSync(storageDir, { recursive: true, force: true });
      console.log(`  ✓ Removed binary storage: ${storageDir}`);
    } catch (e) {
      console.warn(`  ! Could not remove ${storageDir}: ${e.message}`);
    }
  }

  if (purgeData) {
    const isWin = process.platform === 'win32';
    const candidateDirs = [];

    if (isWin) {
      if (process.env.APPDATA) {
        candidateDirs.push(path.join(process.env.APPDATA, 'sumanmovies-tui'));
      }
      if (process.env.LOCALAPPDATA) {
        candidateDirs.push(path.join(process.env.LOCALAPPDATA, 'sumanmovies-tui'));
      }
    } else {
      candidateDirs.push(path.join(home, '.config', 'sumanmovies-tui'));
      candidateDirs.push(path.join(home, '.local', 'share', 'sumanmovies-tui'));
      candidateDirs.push(path.join(home, '.cache', 'sumanmovies-tui'));
    }

    for (const dir of candidateDirs) {
      if (fs.existsSync(dir)) {
        try {
          fs.rmSync(dir, { recursive: true, force: true });
          console.log(`  ✓ Purged data & config: ${dir}`);
        } catch (e) {
          console.warn(`  ! Could not remove ${dir}: ${e.message}`);
        }
      }
    }
  }

  console.log('\n  ✨ SumanMovies local binaries and files uninstalled successfully!');
  console.log('  To completely remove the package from npm, run:');
  console.log('    npm uninstall -g sumanmovies\n');
}

module.exports = {
  getStorageDir,
  findExecutable,
  ensureSumanMoviesBinary,
  ensureMpv,
  ensureYtDlp,
  ensureFfmpeg,
  ensureWezTerm,
  setupAll,
  update,
  uninstall
};

class SumanmoviesTui < Formula
  VERSION = "1.0.2"
  MACOS_SHA256 = "f47817f953123a15b11c3c245d07c7306a02ff099ca46fb5ce82529708518cbe"
  LINUX_X64_SHA256 = "384a75d6a38e77733c03ce982aa00e8217e9e223f1c5201eb5edf6f6efde8f2c"
  LINUX_ARM64_SHA256 = "5be4775f4d54a5ba88b13aea2d4387fbe524daa3ab9a751902f67e0e5a12f5fa"

  desc "Stream movies, shows, anime, and live TV from your terminal"
  homepage "https://github.com/SumanCH8514/SumanMovies-Tui"
  version VERSION
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    url "https://github.com/SumanCH8514/SumanMovies-Tui/releases/download/v#{VERSION}/SumanMovies_macOS_Universal.tar.gz"
    sha256 MACOS_SHA256
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/SumanCH8514/SumanMovies-Tui/releases/download/v#{VERSION}/SumanMovies_Linux_arm64.tar.gz"
      sha256 LINUX_ARM64_SHA256
    else
      url "https://github.com/SumanCH8514/SumanMovies-Tui/releases/download/v#{VERSION}/SumanMovies_Linux_x64.tar.gz"
      sha256 LINUX_X64_SHA256
    end
  end

  def install
    bin.install "sumanmovies-tui"
    bin.install_symlink "sumanmovies-tui" => "sumanmovies"
  end

  test do
    system "#{bin}/sumanmovies-tui", "--version"
  end
end

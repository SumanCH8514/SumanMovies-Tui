class SumanmoviesTui < Formula
  VERSION = "1.0.1"
  MACOS_SHA256 = "92d4181b7c02d1e10d35b602cbc1602257d1fa6b9227df305de123d13d4bfa31"
  LINUX_X64_SHA256 = "ef136e02df801dacf135039bd08a2ac896bd3380df1a88ea52d85368903a4bd9"
  LINUX_ARM64_SHA256 = "ce9205dab01b42ffda33f0873e0eb920a9e32e4b24967c285c55f9ca5fd31358"

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

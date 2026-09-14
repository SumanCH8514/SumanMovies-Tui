class SumanmoviesTui < Formula
  VERSION = "1.0.0"
  MACOS_SHA256 = "438501f9f525f39153300aed532f0ffda37f92327a8cfcc4e3eb749a664ffdca"
  LINUX_X64_SHA256 = "bf4041f8c08f3b9e48454a0c0a476d89ec5ecf75b5f93f2e8b80894bbcdcf17f"
  LINUX_ARM64_SHA256 = "bf33686cba95b30bd633e4e2365f195ede932fe38758386c10cba6e3561f2845"

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

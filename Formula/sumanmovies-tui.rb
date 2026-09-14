class SumanmoviesTui < Formula
  VERSION = "1.0.0"
  MACOS_SHA256 = "1361008cd743e14aadb22bb03b475dd77b16434c3bd27ca60732f3e3155bf151"
  LINUX_X64_SHA256 = "a3479d41e9b92c7a8f257a2207d664ea01e08d69e0ac7360a81f1d3526eb6a4c"
  LINUX_ARM64_SHA256 = "fdc3b59a948f7816a4ce951c795e0c8ddbfd2a9af921b2577b7bd63d72fe4b85"

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

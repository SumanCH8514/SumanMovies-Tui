class MovieboxTui < Formula
  VERSION = "0.1.22"
  MACOS_SHA256 = "2d94e514034fcc9fd4e95ffc9c5b9a86c08152c4e10b5bf62180542bccfb96d5"
  LINUX_X64_SHA256 = "1e907f0f460f9c4fb9c230f6c50fb49d0db72d4e605684f477e091e555fa76ca"
  LINUX_ARM64_SHA256 = "d2ee3578714b9139f836a253f8881163c95d3ca56ebc9517def25c5e9b042398"

  desc "Stream movies, shows, anime, and live TV from your terminal"
  homepage "https://github.com/mesamirh/MovieBox-Tui"
  version VERSION
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    url "https://github.com/mesamirh/MovieBox-Tui/releases/download/v#{VERSION}/MovieBox_macOS_Universal.tar.gz"
    sha256 MACOS_SHA256
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/mesamirh/MovieBox-Tui/releases/download/v#{VERSION}/MovieBox_Linux_arm64.tar.gz"
      sha256 LINUX_ARM64_SHA256
    else
      url "https://github.com/mesamirh/MovieBox-Tui/releases/download/v#{VERSION}/MovieBox_Linux_x64.tar.gz"
      sha256 LINUX_X64_SHA256
    end
  end

  def install
    bin.install "moviebox-tui"
  end

  test do
    system "#{bin}/moviebox-tui", "--version"
  end
end

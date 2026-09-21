class MovieboxTui < Formula
  VERSION = "0.1.23"
  MACOS_SHA256 = "301c8ea2b5f623d8d3ca4a47cf7b8403b413c7edca0d1157b5ec43195229565b"
  LINUX_X64_SHA256 = "4f34942db2d6e51a0055f6ad210ecfdcaac7869b20d478fc82cebc463031dd82"
  LINUX_ARM64_SHA256 = "3e72138dec7af93e6406d259fd1875b2e2a4773ae804dbfc73108ba2718e6ba9"

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
